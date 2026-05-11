use pyo3::prelude::*;
use tokio::task;

pub fn call_python() {
    Python::with_gil(|_| {
        let _ = task::spawn_blocking(|| {});
    });
}
