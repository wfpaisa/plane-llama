// Cierre explícito sin instalar señales en las aplicaciones consumidoras.
use crate::Core;
pub async fn shutdown(core: &Core) {
    core.stop_benchmark();
    core.stop_managed(std::time::Duration::from_secs(3)).await;
}
