use async_session::{Result, Session, SessionStore};

const SESSION_KEY: &str = env!("CARGO_CRATE_NAME");

#[derive(Clone)]
pub struct WorkerKVStore {
    inner: worker::kv::KvStore,
}

impl WorkerKVStore {
    pub fn new() -> WorkerKVStore {
        let inner = worker::kv::KvStore::create(SESSION_KEY).unwrap();
        Self { inner }
    }
}

impl std::fmt::Debug for WorkerKVStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WorkerKVStore").finish()
    }
}

fn map_err(err: worker::kv::KvError) -> anyhow::Error {
    anyhow::anyhow!("{err}")
}

#[async_session::async_trait]
impl SessionStore for WorkerKVStore {
    async fn load_session(&self, cookie_value: String) -> Result<Option<Session>> {
        let id = Session::id_from_cookie_value(&cookie_value)?;
        tracing::trace!("loading session by id `{}`", id);
        Ok(self
            .inner
            .get(&id)
            .json()
            .await
            .map_err(map_err)?
            .and_then(Session::validate))
    }

    async fn store_session(&self, session: Session) -> Result<Option<String>> {
        tracing::trace!("storing session by id `{}`", session.id());

        self.inner
            .put(session.id(), session)
            .map_err(map_err)?
            .execute()
            .await
            .map_err(map_err)?;

        session.reset_data_changed();
        Ok(session.into_cookie_value())
    }

    async fn destroy_session(&self, session: Session) -> Result {
        tracing::trace!("destroying session by id `{}`", session.id());
        self.inner.delete(session.id()).await.map_err(map_err)
    }

    async fn clear_store(&self) -> Result {
        Err(anyhow::anyhow!("Unimplemented"))
    }
}
