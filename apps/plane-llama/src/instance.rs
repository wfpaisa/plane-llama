// Reserva la instancia por sesión y recibe solicitudes para mostrar la ventana.
use std::{
    io::{self, Read, Write},
    os::unix::net::{UnixListener, UnixStream},
    path::{Path, PathBuf},
};
pub struct Instance {
    pub listener: UnixListener,
    path: PathBuf,
}
pub fn acquire(path: &Path) -> io::Result<Option<Instance>> {
    let lock = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(path.with_extension("lock"))?;
    fs2::FileExt::lock_exclusive(&lock)?;
    match UnixStream::connect(path) {
        Ok(mut stream) => {
            stream.write_all(b"show\n")?;
            return Ok(None);
        }
        Err(e)
            if matches!(
                e.kind(),
                io::ErrorKind::NotFound | io::ErrorKind::ConnectionRefused
            ) => {}
        Err(e) => return Err(e),
    }
    // Solo se retira un socket sin servidor; bind decide quién gana una carrera.
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    match UnixListener::bind(path) {
        Ok(listener) => Ok(Some(Instance {
            listener,
            path: path.into(),
        })),
        Err(e) if e.kind() == io::ErrorKind::AddrInUse => {
            UnixStream::connect(path)?.write_all(b"show\n")?;
            Ok(None)
        }
        Err(e) => Err(e),
    }
}
pub fn is_show(stream: &mut UnixStream) -> bool {
    let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(1)));
    let mut command = [0; 5];
    stream.read_exact(&mut command).is_ok() && &command == b"show\n"
}
impl Drop for Instance {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn concurrent_start_has_only_one_owner() {
        let dir = tempfile::tempdir().unwrap();
        let path = std::sync::Arc::new(dir.path().join("app.sock"));
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(8));
        let threads: Vec<_> = (0..8)
            .map(|_| {
                let path = path.clone();
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    barrier.wait();
                    acquire(&path).unwrap()
                })
            })
            .collect();
        let owners: Vec<_> = threads
            .into_iter()
            .filter_map(|t| t.join().unwrap())
            .collect();
        assert_eq!(owners.len(), 1);
        for _ in 0..7 {
            let (mut stream, _) = owners[0].listener.accept().unwrap();
            assert!(is_show(&mut stream));
        }
    }
    #[test]
    fn second_instance_requests_show_and_stale_socket_recovers() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("app.sock");
        let first = acquire(&path).unwrap().unwrap();
        assert!(acquire(&path).unwrap().is_none());
        let (mut stream, _) = first.listener.accept().unwrap();
        assert!(is_show(&mut stream));
        drop(first);
        let stale = UnixListener::bind(&path).unwrap();
        drop(stale);
        assert!(acquire(&path).unwrap().is_some());
    }
}
