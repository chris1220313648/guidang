use async_stream::stream;
use color_eyre::eyre::eyre;
use color_eyre::{eyre::WrapErr, Result};
use flume::{Receiver, Sender};
use futures::StreamExt;
use proto::server_message::Msg;
use proto::{
    client_message::{ClientCode, ClientInfo},
    controller_service_client::ControllerServiceClient,
    server_message::RunScript,
    ClientMessage, ServerMessage,
};
use std::result::Result as StdResult;
use tokio::task::JoinHandle;
use tonic::Streaming;
use tonic::{metadata::MetadataValue, transport::channel::Channel, Request, Status};
use tracing::{error, info};

const RE_VERSION: &str = "re-version";
const CLIENT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub struct Client {
    pub client: ControllerServiceClient<Channel>,
    pub tasks: Vec<JoinHandle<()>>,
    pub id: u32,
    pub rx: Receiver<RunScript>,
}

impl Client {
    pub async fn try_connect(url: String) -> Result<Client> {
        info!("Connecting to server {}", url);
        let client = ControllerServiceClient::connect(url).await?;
        let main_client = client.clone();
        let mut tasks = Vec::new();
        let (tx, rx) = flume::bounded(10);
        let (id, stream) = connect(main_client).await?;
        info!("Connected!");
        let handle = tokio::spawn(async move {
            if let Err(e) = run(stream, tx).await {
                error!(error =? e, "Connection to controller get a error");
            }
        });
        tasks.push(handle);
        Ok(Client {
            client,
            tasks,
            id,
            rx,
        })
    }
}

async fn connect(
    mut client: ControllerServiceClient<Channel>,
) -> Result<(u32, Streaming<ServerMessage>)> {
    let client_stream = stream! {
        yield ClientMessage {
            code: ClientCode::Connect as i32,
            info: Some(ClientInfo {
                max_job: 0
            })
        };
        loop {
            yield ClientMessage {
                code: ClientCode::Continue as i32,
                info: None
            }
        }
    };
    let mut request = Request::new(client_stream);
    request
        .metadata_mut()
        .insert(RE_VERSION, MetadataValue::from_static(CLIENT_VERSION));

    let mut stream = client
        .run(request)
        .await
        .wrap_err("Got error from server")?
        .into_inner();
    let msg = stream.next().await;
    let executor_id = handle_first_message(msg)?;
    Ok((executor_id, stream))
}

fn handle_first_message(msg: Option<StdResult<ServerMessage, Status>>) -> Result<u32> {
    match msg {
        None => Err(eyre!("Got None on first message")),
        Some(msg) => {
            let msg = msg
                .wrap_err("Got error on first message")?
                .msg
                .ok_or_else(|| eyre!("Got None on ServerMessage"))?;
            match msg {
                Msg::Connected(c) => Ok(c.executor_id),
                Msg::Disconnect(d) => Err(eyre!("Got Disconnect on first message: {:?}", d)),
                Msg::Script(s) => Err(eyre!("Got Script on first message: {:?}", s)),
            }
        }
    }
}

async fn run(mut stream: Streaming<ServerMessage>, tx: Sender<RunScript>) -> Result<()> {
    loop {
        match stream.next().await {
            Some(msg) => {
                let msg = msg
                    .wrap_err("Got error when receive message")?
                    .msg
                    .ok_or_else(|| eyre!("Got None on ServerMessage"))?;
                match msg {
                    Msg::Connected(_) => return Err(eyre!("Unexpect Connected on ServerMessage")),
                    Msg::Disconnect(d) => {
                        info!(reason =? d.reason(), "Disconnect");
                        break Ok(());
                    }
                    Msg::Script(r) => {
                        tx.send_async(r).await?;
                    }
                }
            }
            None => return Err(eyre!("Unexpect disconnect")),
        }
    }
}
