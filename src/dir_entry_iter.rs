use crate::{ClientState, DirEntry, Error, Parallelism, ReadChildren, Result};
use crossbeam::channel::{self, Receiver, Sender};
use rayon::ThreadPoolBuilder;
use std::cmp::Ordering as CmpOrdering;
use std::collections::VecDeque;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

type ProcessReadDirFunction<C> = dyn Fn(Option<usize>, &Path, &mut <C as ClientState>::ReadDirState, &mut Vec<Result<DirEntry<C>>>)
    + Send
    + Sync
    + 'static;

pub(crate) struct WalkDirOptions<C: ClientState> {
    pub sort: bool,
    pub min_depth: usize,
    pub max_depth: usize,
    pub skip_hidden: bool,
    pub follow_links: bool,
    pub parallelism: Parallelism,
    pub root_read_dir_state: C::ReadDirState,
    pub process_read_dir: Option<Arc<ProcessReadDirFunction<C>>>,
}

impl<C: ClientState> Clone for WalkDirOptions<C> {
    fn clone(&self) -> Self {
        WalkDirOptions {
            sort: self.sort,
            min_depth: self.min_depth,
            max_depth: self.max_depth,
            skip_hidden: self.skip_hidden,
            follow_links: self.follow_links,
            parallelism: self.parallelism.clone(),
            root_read_dir_state: self.root_read_dir_state.clone(),
            process_read_dir: self.process_read_dir.clone(),
        }
    }
}

/// DirEntry iterator from `WalkDir.into_iter()`.
///
/// Yields entries from recursive traversal of filesystem.
pub struct DirEntryIter<C: ClientState> {
    /// Stack of entries to process (for serial mode)
    stack: VecDeque<Result<DirEntry<C>>>,
    /// Options for the walk
    options: WalkDirOptions<C>,
    /// Initial error to yield (if any)
    initial_error: Option<Error>,
    /// For parallel mode
    receiver: Option<Receiver<Result<DirEntry<C>>>>,
}

impl<C: ClientState> DirEntryIter<C> {
    pub(crate) fn new(root: PathBuf, options: WalkDirOptions<C>) -> Result<Self> {
        let is_serial = matches!(options.parallelism, Parallelism::Serial);
        
        // Collect root entry
        let root_entry = Self::create_root_entry(&root, &options)?;
        
        if is_serial {
            let mut stack = VecDeque::new();
            stack.push_back(Ok(root_entry));
            Ok(DirEntryIter {
                stack,
                options,
                initial_error: None,
                receiver: None,
            })
        } else {
            // Parallel mode: set up channel and spawn processing
            let (sender, receiver) = channel::unbounded();
            
            let options_for_parallel = options.clone();
            
            // Spawn parallel processing - sender is moved and will be dropped when done
            match &options.parallelism {
                Parallelism::RayonDefaultPool { busy_timeout } => {
                    let timeout = *busy_timeout;
                    let start = Instant::now();
                    
                    // Spawn a task that will monitor for timeout
                    let (timeout_sender, timeout_receiver) = channel::bounded(1);
                    rayon::spawn(move || {
                        // Signal that we've started executing
                        let _ = timeout_sender.send(());
                        if start.elapsed() > timeout {
                            let _ = sender.send(Err(Error::busy()));
                            return;
                        }
                        Self::process_parallel(root_entry, sender, options_for_parallel);
                    });
                    
                    // Wait for either the task to start or timeout to expire
                    if timeout_receiver.recv_timeout(timeout).is_err() {
                        // Timeout expired before task could start - thread pool is busy
                        return Err(Error::busy());
                    }
                }
                Parallelism::RayonExistingPool { pool, busy_timeout } => {
                    let start = Instant::now();
                    let pool = pool.clone();
                    let timeout_clone = *busy_timeout;
                    
                    if let Some(timeout) = timeout_clone {
                        let (timeout_sender, timeout_receiver) = channel::bounded(1);
                        pool.spawn(move || {
                            let _ = timeout_sender.send(());
                            if start.elapsed() > timeout {
                                let _ = sender.send(Err(Error::busy()));
                                return;
                            }
                            Self::process_parallel(root_entry, sender, options_for_parallel);
                        });
                        
                        if timeout_receiver.recv_timeout(timeout).is_err() {
                            return Err(Error::busy());
                        }
                    } else {
                        pool.spawn(move || {
                            Self::process_parallel(root_entry, sender, options_for_parallel);
                        });
                    }
                }
                Parallelism::RayonNewPool(num_threads) => {
                    let pool = ThreadPoolBuilder::new()
                        .num_threads(num_threads.clone())
                        .build()
                        .map_err(|e| Error::from_io(0, std::io::Error::new(std::io::ErrorKind::Other, e)))?;
                    
                    pool.spawn(move || {
                        Self::process_parallel(root_entry, sender, options_for_parallel);
                    });
                }
                Parallelism::Serial => unreachable!(),
            }
            
            // Don't store the sender - it will be dropped when the spawned task completes
            Ok(DirEntryIter {
                stack: VecDeque::new(),
                options,
                initial_error: None,
                receiver: Some(receiver),
            })
        }
    }
    
    pub(crate) fn with_error(error: Error) -> Self {
        DirEntryIter {
            stack: VecDeque::new(),
            options: WalkDirOptions {
                sort: false,
                min_depth: 0,
                max_depth: std::usize::MAX,
                skip_hidden: true,
                follow_links: false,
                parallelism: Parallelism::Serial,
                root_read_dir_state: C::ReadDirState::default(),
                process_read_dir: None,
            },
            initial_error: Some(error),
            receiver: None,
        }
    }
    
    fn create_root_entry(root: &Path, options: &WalkDirOptions<C>) -> Result<DirEntry<C>> {
        // Always check if it's a symlink first
        let symlink_metadata = fs::symlink_metadata(root)
            .map_err(|e| Error::from_path(0, root.to_path_buf(), e))?;
        
        let is_symlink = symlink_metadata.file_type().is_symlink();
        
        let file_type = if is_symlink && options.follow_links {
            // Follow the symlink to get the target type
            match fs::metadata(root) {
                Ok(m) => m.file_type(),
                Err(e) => return Err(Error::from_path(0, root.to_path_buf(), e)),
            }
        } else {
            symlink_metadata.file_type()
        };
        
        let file_name = root.file_name()
            .map(|n| n.to_os_string())
            .unwrap_or_else(|| root.as_os_str().to_os_string());
        
        let parent_path = root.parent()
            .unwrap_or_else(|| Path::new(""))
            .to_path_buf();
        
        // Determine if we should read children: check if it's a directory OR a symlink to a directory
        let should_read_children = if file_type.is_dir() {
            true
        } else if is_symlink {
            // Check if the symlink points to a directory
            match fs::metadata(root) {
                Ok(m) => m.file_type().is_dir(),
                Err(_) => false,
            }
        } else {
            false
        };
        
        let read_children = if should_read_children && 0 < options.max_depth {
            Some(ReadChildren {
                path: Arc::from(root),
                error: None,
                client_read_state: Some(options.root_read_dir_state.clone()),
            })
        } else {
            None
        };
        
        // Initialize ancestors with root's path if we're following links and it's a directory
        // Store the actual path, not canonical - we'll canonicalize when comparing
        let follow_link_ancestors = if options.follow_links && file_type.is_dir() {
            Arc::new(vec![Arc::from(root)])
        } else {
            Arc::new(Vec::new())
        };
        
        Ok(DirEntry {
            depth: 0,
            file_name,
            file_type,
            client_state: C::DirEntryState::default(),
            parent_path: Arc::from(parent_path.as_path()),
            read_children,
            follow_link: is_symlink,
            followed_link: is_symlink && options.follow_links,
            follow_link_ancestors,
        })
    }
    
    fn process_parallel(
        root: DirEntry<C>,
        sender: Sender<Result<DirEntry<C>>>,
        options: WalkDirOptions<C>,
    ) {
        let mut stack = VecDeque::new();
        stack.push_back(Ok(root));
        
        while let Some(entry_result) = stack.pop_front() {
            match entry_result {
                Ok(mut entry) => {
                    let should_send = entry.depth >= options.min_depth;
                    let children_opt = entry.read_children.take();
                    
                    if let Some(children_spec) = children_opt.clone() {
                        let mut children = Self::read_dir_entries(&children_spec, &entry, &options);
                        
                        // Sort BEFORE process_read_dir if sort is enabled
                        if options.sort {
                            children.sort_by(|a, b| {
                                Self::entry_sort_cmp(a, b)
                            });
                        }
                        
                        if let Some(ref process) = options.process_read_dir {
                            let mut state = children_spec.client_read_state.clone()
                                .unwrap_or_else(|| C::ReadDirState::default());
                            process(Some(entry.depth), &entry.path(), &mut state, &mut children);
                        }
                        
                        // Check if read_dir failed
                        let read_dir_failed = children.len() == 1 && 
                            children.first().map_or(false, |r| {
                                if let Err(ref e) = r {
                                    e.path() == Some(entry.path().as_path())
                                } else {
                                    false
                                }
                            });
                        
                        // Restore read_children with error if read_dir failed
                        entry.read_children = Some(ReadChildren {
                            path: children_spec.path.clone(),
                            error: if read_dir_failed {
                                children.first().and_then(|r| r.as_ref().err()).cloned()
                            } else {
                                None
                            },
                            client_read_state: children_spec.client_read_state.clone(),
                        });
                        
                        if should_send {
                            if sender.send(Ok(entry.clone())).is_err() {
                                return;
                            }
                        }
                        
                        if !read_dir_failed {
                            // Add children to stack in reverse order for DFS
                            for child in children.into_iter().rev() {
                                stack.push_front(child);
                            }
                        }
                    } else {
                        // No children to process
                        entry.read_children = children_opt;
                        if should_send {
                            if sender.send(Ok(entry.clone())).is_err() {
                                return;
                            }
                        }
                    }
                }
                Err(e) => {
                    if sender.send(Err(e)).is_err() {
                        return;
                    }
                }
            }
        }
    }
    
    fn read_dir_entries(
        children_spec: &ReadChildren<C>,
        parent: &DirEntry<C>,
        options: &WalkDirOptions<C>,
    ) -> Vec<Result<DirEntry<C>>> {
        let path = &children_spec.path;
        let read_dir = match fs::read_dir(path.as_ref()) {
            Ok(rd) => rd,
            Err(e) => {
                return vec![Err(Error::from_path(
                    parent.depth + 1,
                    path.to_path_buf(),
                    e,
                ))];
            }
        };
        
        let mut entries = Vec::new();
        let child_depth = parent.depth + 1;
        
        'outer_loop: for entry in read_dir {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    entries.push(Err(Error::from_io(child_depth, e)));
                    continue;
                }
            };
            
            let file_name = entry.file_name();
            
            // Skip hidden files if requested
            if options.skip_hidden {
                if let Some(s) = file_name.to_str() {
                    if s.starts_with('.') {
                        continue;
                    }
                }
            }
            
            let file_type = match entry.file_type() {
                Ok(ft) => ft,
                Err(e) => {
                    entries.push(Err(Error::from_path(
                        child_depth,
                        entry.path(),
                        e,
                    )));
                    continue;
                }
            };
            
            let is_symlink = file_type.is_symlink();
            
            let (actual_file_type, new_ancestors) = if is_symlink && options.follow_links {
                match fs::metadata(entry.path()) {
                    Ok(m) => {
                        let target_type = m.file_type();
                        if target_type.is_dir() {
                            // Check for loops
                            let canonical = match entry.path().canonicalize() {
                                Ok(c) => c,
                                Err(e) => {
                                    entries.push(Err(Error::from_path(
                                        child_depth,
                                        entry.path(),
                                        e,
                                    )));
                                    continue;
                                }
                            };
                            
                            for ancestor in parent.follow_link_ancestors.iter() {
                                // Canonicalize ancestor for comparison
                                if let Ok(anc_canonical) = ancestor.as_ref().canonicalize() {
                                    if canonical == anc_canonical {
                                        entries.push(Err(Error::from_loop(
                                            child_depth,
                                            ancestor.to_path_buf(),
                                            entry.path(),
                                        )));
                                        // Don't add this entry to the list - it's a loop
                                        continue 'outer_loop;
                                    }
                                }
                            }
                            
                            let mut new_anc = (*parent.follow_link_ancestors).clone();
                            // Store the symlink path (not the canonical target path)
                            new_anc.push(Arc::from(entry.path().as_path()));
                            (target_type, Arc::new(new_anc))
                        } else {
                            (target_type, parent.follow_link_ancestors.clone())
                        }
                    }
                    Err(e) => {
                        entries.push(Err(Error::from_path(
                            child_depth,
                            entry.path(),
                            e,
                        )));
                        continue;
                    }
                }
            } else {
                // Regular file or directory (not a symlink), keep parent's ancestors
                (file_type, parent.follow_link_ancestors.clone())
            };
            
            // For directories (when follow_links is on), add current dir to ancestors for its children
            // This is the ancestors list that THIS entry's children will see
            // Store the actual path, not canonical - we'll canonicalize when comparing
            let final_ancestors = if actual_file_type.is_dir() && options.follow_links {
                let mut new_anc = (*new_ancestors).clone();
                new_anc.push(Arc::from(entry.path().as_path()));
                Arc::new(new_anc)
            } else {
                new_ancestors
            };
            
            // Determine if we should read children
            // Only read children if it's actually a directory (not a symlink to a directory)
            let should_read_children = actual_file_type.is_dir();
            
            let read_children = if should_read_children && child_depth < options.max_depth {
                let child_state = children_spec.client_read_state.clone();
                Some(ReadChildren {
                    path: Arc::from(entry.path().as_path()),
                    error: None,
                    client_read_state: child_state,
                })
            } else {
                None
            };
            
            entries.push(Ok(DirEntry {
                depth: child_depth,
                file_name,
                file_type: actual_file_type,
                client_state: C::DirEntryState::default(),
                parent_path: parent.path().into(),
                read_children,
                follow_link: is_symlink,
                followed_link: is_symlink && options.follow_links,
                follow_link_ancestors: final_ancestors,
            }));
        }
        
        entries
    }
    
    fn entry_sort_cmp(a: &Result<DirEntry<C>>, b: &Result<DirEntry<C>>) -> CmpOrdering {
        match (a, b) {
            (Ok(a), Ok(b)) => a.file_name.cmp(&b.file_name),
            (Ok(_), Err(_)) => CmpOrdering::Less,
            (Err(_), Ok(_)) => CmpOrdering::Greater,
            (Err(_), Err(_)) => CmpOrdering::Equal,
        }
    }
}

impl<C: ClientState> Iterator for DirEntryIter<C> {
    type Item = Result<DirEntry<C>>;
    
    fn next(&mut self) -> Option<Self::Item> {
        // Return initial error if present
        if let Some(err) = self.initial_error.take() {
            return Some(Err(err));
        }
        
        // Parallel mode
        if let Some(ref receiver) = self.receiver {
            return receiver.recv().ok();
        }
        
        // Serial mode
        while let Some(entry_result) = self.stack.pop_front() {
            match entry_result {
                Ok(mut entry) => {
                    let should_yield = entry.depth >= self.options.min_depth;
                    let children_opt = entry.read_children.take();
                    
                    if should_yield {
                        entry.read_children = children_opt.clone();
                        
                        // Process children if needed
                        if let Some(children_spec) = children_opt {
                            let mut children = Self::read_dir_entries(&children_spec, &entry, &self.options);
                            
                            // Sort BEFORE process_read_dir if sort is enabled
                            if self.options.sort {
                                children.sort_by(Self::entry_sort_cmp);
                            }
                            
                            if let Some(ref process) = self.options.process_read_dir {
                                let mut state = children_spec.client_read_state.clone()
                                    .unwrap_or_else(|| C::ReadDirState::default());
                                process(Some(entry.depth), &entry.path(), &mut state, &mut children);
                            }
                            
                            // Check if read_dir itself failed: single error with path == directory path
                            // vs individual entry errors or loop errors (path != directory path)
                            let read_dir_failed = children.len() == 1 && 
                                children.first().map_or(false, |r| {
                                    if let Err(ref e) = r {
                                        e.path() == Some(entry.path().as_path())
                                    } else {
                                        false
                                    }
                                });
                            
                            if read_dir_failed {
                                // read_dir failed - store error but don't yield it
                                if let Some(ref mut rc) = entry.read_children {
                                    if let Some(Err(ref e)) = children.first() {
                                        rc.error = Some(e.clone());
                                    }
                                }
                                // Don't add to stack - don't yield the error
                            } else {
                                // read_dir succeeded, might have individual entry errors
                                // Add children to front of stack in reverse for DFS
                                for child in children.into_iter().rev() {
                                    self.stack.push_front(child);
                                }
                            }
                        }
                        
                        return Some(Ok(entry));
                    } else {
                        // Process children but don't yield
                        if let Some(children_spec) = children_opt {
                            let mut children = Self::read_dir_entries(&children_spec, &entry, &self.options);
                            
                            // Sort BEFORE process_read_dir if sort is enabled
                            if self.options.sort {
                                children.sort_by(Self::entry_sort_cmp);
                            }
                            
                            if let Some(ref process) = self.options.process_read_dir {
                                let mut state = children_spec.client_read_state.clone()
                                    .unwrap_or_else(|| C::ReadDirState::default());
                                process(Some(entry.depth), &entry.path(), &mut state, &mut children);
                            }
                            
                            // Check if read_dir failed - if so, skip adding children
                            let read_dir_failed = children.len() == 1 && 
                                children.first().map_or(false, |r| {
                                    if let Err(ref e) = r {
                                        e.path() == Some(entry.path().as_path())
                                    } else {
                                        false
                                    }
                                });
                            
                            if !read_dir_failed {
                                for child in children.into_iter().rev() {
                                    self.stack.push_front(child);
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    if e.depth() >= self.options.min_depth {
                        return Some(Err(e));
                    }
                }
            }
        }
        
        None
    }
}


