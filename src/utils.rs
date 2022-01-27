use crate::prelude::*;

pub fn ensure_dir_exists(what: &str, path: &Path) -> IOResult<()> {
    let trace_msg = format!(
        "{}(what={:?}, path={:?})",
        function!(),
        what,
        path);
    trace!("+{}", trace_msg);
    if !path.exists() {
        debug!(
            "{} does not exist, will try to make it if parent exists",
            what
        );
        let parent = path.parent();
        debug!("{}.parent() = {:?}", what, parent);
        if let Some(parent) = parent {
            if parent.exists() {
                match fs::create_dir(path) {
                    Ok(_) => {
                        debug!("Created {}", what);
                        trace!(
                            "-{} -> {:?}",
                            trace_msg,
                            true
                        );
                        return Ok(());
                    }
                    Err(err) => {
                        error!("Failed to make {}: {:?}", what, err);
                        trace!(
                            "-{} -> {:?}",
                            trace_msg,
                            false
                        );
                        return Err(err);
                    }
                }
            } else {
                debug!("{}'s parent does not exist", what);
                error!("{} {:?} does not exists", what, path);
                trace!(
                    "-{} -> {:?}",
                    trace_msg,
                    false
                );
                return Err(IOError::new(IOErrorKind::NotFound, ""));
            }
        } else {
            error!("Failed to get {}'s parent", what);
            error!("{} {:?} does not exists", what, path);
            trace!(
                "-{} -> {:?}",
                trace_msg,
                false
            );
            return Err(IOError::new(
                IOErrorKind::NotFound,
                format!("{:?} is not a directory", path),
            ));
        }
    } else if !path.is_dir() {
        error!("{:?} is not a directory", path);
        trace!(
            "-{} -> {:?}",
            trace_msg,
            false
        );
        return Err(IOError::new(
            IOErrorKind::NotFound,
            format!("{:?} is not a directory", path),
        ));
    } else {
        debug!("Great! {} exists and is a folder", what);
        trace!(
            "-{} -> {:?}",
            trace_msg,
            true
        );
        return Ok(());
    }
}
