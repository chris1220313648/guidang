pub mod kubeapi;
pub mod mqtt;
pub mod webhook;

#[cfg(test)]
pub(crate) mod test {
    use std::marker::PhantomData;
    use std::time::Duration;

    use crate::api::Script;
    use crate::scheduler::ResourceIndex;
    use flume::Sender;
    use tokio::task::JoinHandle;

    pub(crate) async fn test_triger(
        name: String,
        namespace: String,
    ) -> (Sender<ResourceIndex<Script>>, JoinHandle<bool>) {
        let (tx, rx) = flume::bounded(3);
        let handle = tokio::spawn(async move {
            let ri: ResourceIndex<Script> = rx.recv_async().await.unwrap();
            ri.name == name && ri.namespace == namespace
        });
        (tx, handle)
    }

    #[tokio::test]
    async fn test_drop() {
        let name = String::from("test_name");
        let namespace = String::from("test_namespace");
        let (tx, handle) = test_triger(name.clone(), namespace.clone()).await;
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(3)).await;
            tx.send(ResourceIndex {
                namespace,
                name,
                api: PhantomData,
            })
            .unwrap();
        });
        let result = handle.await.unwrap();
        assert!(result)
    }

    #[tokio::test]
    async fn test_broken_triger() {
        let name = String::from("test_name");
        let namespace = String::from("test_namespace");
        let (tx, handle) = test_triger(name.clone(), namespace.clone()).await;
        tx.send(ResourceIndex {
            namespace: String::new(),
            name: String::new(),
            api: PhantomData,
        })
        .unwrap();
        let result = handle.await.unwrap();
        assert!(!result)
    }

    #[tokio::test]
    async fn test_ok() {
        let namespace = String::from("test_namespace");
        let name = String::from("test_name");
        let (tx, handle) = test_triger(name.clone(), namespace.clone()).await;
        tx.send(ResourceIndex {
            namespace,
            name,
            api: PhantomData,
        })
        .unwrap();
        let result = handle.await.unwrap();
        assert!(result)
    }
}
