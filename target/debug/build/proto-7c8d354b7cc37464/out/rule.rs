/// Controller-Executor connection Executor Side Message
/// Executor first send code = Connect and fill info struct
/// Then send Continue/Disconnect/Pause
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ClientMessage {
    #[prost(enumeration="client_message::ClientCode", tag="1")]
    pub code: i32,
    #[prost(message, optional, tag="2")]
    pub info: ::core::option::Option<client_message::ClientInfo>,
}
/// Nested message and enum types in `ClientMessage`.
pub mod client_message {
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct ClientInfo {
        #[prost(uint32, tag="1")]
        pub max_job: u32,
    }
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
    #[repr(i32)]
    pub enum ClientCode {
        Continue = 0,
        Connect = 1,
        Disconnect = 2,
    }
}
/// Controller-Executor connection Executor Side Message
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ServerMessage {
    #[prost(oneof="server_message::Msg", tags="1, 2, 3")]
    pub msg: ::core::option::Option<server_message::Msg>,
}
/// Nested message and enum types in `ServerMessage`.
pub mod server_message {
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct Connected {
        #[prost(uint32, tag="1")]
        pub executor_id: u32,
    }
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct Disconnect {
        #[prost(enumeration="disconnect::DisconnectReason", tag="1")]
        pub reason: i32,
    }
    /// Nested message and enum types in `Disconnect`.
    pub mod disconnect {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
        #[repr(i32)]
        pub enum DisconnectReason {
            Unknown = 0,
            ClientExit = 1,
            ServerExit = 2,
            Unneeded = 3,
        }
    }
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct RunScript {
        #[prost(uint32, tag="1")]
        pub script_id: u32,
        #[prost(message, optional, tag="2")]
        pub manifest: ::core::option::Option<run_script::Manifest>,
        #[prost(map="string, message", tag="3")]
        pub readable: ::std::collections::HashMap<::prost::alloc::string::String, run_script::ReadDevice>,
        #[prost(map="string, message", tag="4")]
        pub writable: ::std::collections::HashMap<::prost::alloc::string::String, run_script::WriteDevice>,
        #[prost(map="string, string", tag="5")]
        pub env: ::std::collections::HashMap<::prost::alloc::string::String, ::prost::alloc::string::String>,
        #[prost(enumeration="super::QosPolicy", tag="6")]
        pub default_qos: i32,
    }
    /// Nested message and enum types in `RunScript`.
    pub mod run_script {
        #[derive(Clone, PartialEq, ::prost::Message)]
        pub struct Manifest {
            /// script type
            #[prost(enumeration="manifest::ScriptType", tag="1")]
            pub script_type: i32,
            /// package name
            #[prost(string, tag="2")]
            pub package_name: ::prost::alloc::string::String,
            /// version of package
            #[prost(string, tag="3")]
            pub package_version: ::prost::alloc::string::String,
            /// override the default script package register
            #[prost(string, tag="4")]
            pub register: ::prost::alloc::string::String,
        }
        /// Nested message and enum types in `Manifest`.
        pub mod manifest {
            #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
            #[repr(i32)]
            pub enum ScriptType {
                Wasm = 0,
                Js = 1,
                Native = 2,
                Standalone = 3,
            }
        }
        #[derive(Clone, PartialEq, ::prost::Message)]
        pub struct ReadDevice {
            #[prost(string, tag="1")]
            pub name: ::prost::alloc::string::String,
            #[prost(map="string, string", tag="2")]
            pub status: ::std::collections::HashMap<::prost::alloc::string::String, ::prost::alloc::string::String>,
        }
        #[derive(Clone, PartialEq, ::prost::Message)]
        pub struct WriteDevice {
            #[prost(string, tag="1")]
            pub name: ::prost::alloc::string::String,
        }
    }
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Msg {
        #[prost(message, tag="1")]
        Connected(Connected),
        #[prost(message, tag="2")]
        Disconnect(Disconnect),
        #[prost(message, tag="3")]
        Script(RunScript),
    }
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ScriptStatus {
    #[prost(uint32, tag="1")]
    pub script_id: u32,
    #[prost(message, optional, tag="2")]
    pub start: ::core::option::Option<::prost_types::Timestamp>,
    #[prost(message, optional, tag="3")]
    pub duration: ::core::option::Option<::prost_types::Duration>,
    #[prost(enumeration="script_status::ScriptStatusCode", tag="4")]
    pub code: i32,
    #[prost(string, tag="5")]
    pub message: ::prost::alloc::string::String,
}
/// Nested message and enum types in `ScriptStatus`.
pub mod script_status {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
    #[repr(i32)]
    pub enum ScriptStatusCode {
        Ok = 0,
        Crash = 1,
        Unknown = 3,
    }
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct UpdateDevice {
    #[prost(uint32, tag="1")]
    pub script_id: u32,
    #[prost(string, tag="2")]
    pub name: ::prost::alloc::string::String,
    #[prost(map="string, string", tag="3")]
    pub desired: ::std::collections::HashMap<::prost::alloc::string::String, ::prost::alloc::string::String>,
    #[prost(enumeration="QosPolicy", tag="4")]
    pub qos: i32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum QosPolicy {
    OnlyOnce = 0,
    AtMostOnce = 1,
    AtLeastOnce = 2,
}
/// Generated client implementations.
pub mod controller_service_client {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use tonic::codegen::*;
    #[derive(Debug, Clone)]
    pub struct ControllerServiceClient<T> {
        inner: tonic::client::Grpc<T>,
    }
    impl ControllerServiceClient<tonic::transport::Channel> {
        /// Attempt to create a new client by connecting to a given endpoint.
        pub async fn connect<D>(dst: D) -> Result<Self, tonic::transport::Error>
        where
            D: std::convert::TryInto<tonic::transport::Endpoint>,
            D::Error: Into<StdError>,
        {
            let conn = tonic::transport::Endpoint::new(dst)?.connect().await?;
            Ok(Self::new(conn))
        }
    }
    impl<T> ControllerServiceClient<T>
    where
        T: tonic::client::GrpcService<tonic::body::BoxBody>,
        T::Error: Into<StdError>,
        T::ResponseBody: Body<Data = Bytes> + Send + 'static,
        <T::ResponseBody as Body>::Error: Into<StdError> + Send,
    {
        pub fn new(inner: T) -> Self {
            let inner = tonic::client::Grpc::new(inner);
            Self { inner }
        }
        pub fn with_interceptor<F>(
            inner: T,
            interceptor: F,
        ) -> ControllerServiceClient<InterceptedService<T, F>>
        where
            F: tonic::service::Interceptor,
            T::ResponseBody: Default,
            T: tonic::codegen::Service<
                http::Request<tonic::body::BoxBody>,
                Response = http::Response<
                    <T as tonic::client::GrpcService<tonic::body::BoxBody>>::ResponseBody,
                >,
            >,
            <T as tonic::codegen::Service<
                http::Request<tonic::body::BoxBody>,
            >>::Error: Into<StdError> + Send + Sync,
        {
            ControllerServiceClient::new(InterceptedService::new(inner, interceptor))
        }
        /// Compress requests with `gzip`.
        ///
        /// This requires the server to support it otherwise it might respond with an
        /// error.
        #[must_use]
        pub fn send_gzip(mut self) -> Self {
            self.inner = self.inner.send_gzip();
            self
        }
        /// Enable decompressing responses with `gzip`.
        #[must_use]
        pub fn accept_gzip(mut self) -> Self {
            self.inner = self.inner.accept_gzip();
            self
        }
        pub async fn run(
            &mut self,
            request: impl tonic::IntoStreamingRequest<Message = super::ClientMessage>,
        ) -> Result<
            tonic::Response<tonic::codec::Streaming<super::ServerMessage>>,
            tonic::Status,
        > {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/rule.ControllerService/run",
            );
            self.inner.streaming(request.into_streaming_request(), path, codec).await
        }
        pub async fn update_script_status(
            &mut self,
            request: impl tonic::IntoRequest<super::ScriptStatus>,
        ) -> Result<tonic::Response<()>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/rule.ControllerService/update_script_status",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        pub async fn update_device_desired(
            &mut self,
            request: impl tonic::IntoRequest<super::UpdateDevice>,
        ) -> Result<tonic::Response<()>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/rule.ControllerService/update_device_desired",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
    }
}
/// Generated server implementations.
pub mod controller_service_server {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use tonic::codegen::*;
    ///Generated trait containing gRPC methods that should be implemented for use with ControllerServiceServer.
    #[async_trait]
    pub trait ControllerService: Send + Sync + 'static {
        ///Server streaming response type for the run method.
        type runStream: futures_core::Stream<
                Item = Result<super::ServerMessage, tonic::Status>,
            >
            + Send
            + 'static;
        async fn run(
            &self,
            request: tonic::Request<tonic::Streaming<super::ClientMessage>>,
        ) -> Result<tonic::Response<Self::runStream>, tonic::Status>;
        async fn update_script_status(
            &self,
            request: tonic::Request<super::ScriptStatus>,
        ) -> Result<tonic::Response<()>, tonic::Status>;
        async fn update_device_desired(
            &self,
            request: tonic::Request<super::UpdateDevice>,
        ) -> Result<tonic::Response<()>, tonic::Status>;
    }
    #[derive(Debug)]
    pub struct ControllerServiceServer<T: ControllerService> {
        inner: _Inner<T>,
        accept_compression_encodings: (),
        send_compression_encodings: (),
    }
    struct _Inner<T>(Arc<T>);
    impl<T: ControllerService> ControllerServiceServer<T> {
        pub fn new(inner: T) -> Self {
            Self::from_arc(Arc::new(inner))
        }
        pub fn from_arc(inner: Arc<T>) -> Self {
            let inner = _Inner(inner);
            Self {
                inner,
                accept_compression_encodings: Default::default(),
                send_compression_encodings: Default::default(),
            }
        }
        pub fn with_interceptor<F>(
            inner: T,
            interceptor: F,
        ) -> InterceptedService<Self, F>
        where
            F: tonic::service::Interceptor,
        {
            InterceptedService::new(Self::new(inner), interceptor)
        }
    }
    impl<T, B> tonic::codegen::Service<http::Request<B>> for ControllerServiceServer<T>
    where
        T: ControllerService,
        B: Body + Send + 'static,
        B::Error: Into<StdError> + Send + 'static,
    {
        type Response = http::Response<tonic::body::BoxBody>;
        type Error = std::convert::Infallible;
        type Future = BoxFuture<Self::Response, Self::Error>;
        fn poll_ready(
            &mut self,
            _cx: &mut Context<'_>,
        ) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }
        fn call(&mut self, req: http::Request<B>) -> Self::Future {
            let inner = self.inner.clone();
            match req.uri().path() {
                "/rule.ControllerService/run" => {
                    #[allow(non_camel_case_types)]
                    struct runSvc<T: ControllerService>(pub Arc<T>);
                    impl<
                        T: ControllerService,
                    > tonic::server::StreamingService<super::ClientMessage>
                    for runSvc<T> {
                        type Response = super::ServerMessage;
                        type ResponseStream = T::runStream;
                        type Future = BoxFuture<
                            tonic::Response<Self::ResponseStream>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<
                                tonic::Streaming<super::ClientMessage>,
                            >,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move { (*inner).run(request).await };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = runSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.streaming(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/rule.ControllerService/update_script_status" => {
                    #[allow(non_camel_case_types)]
                    struct update_script_statusSvc<T: ControllerService>(pub Arc<T>);
                    impl<
                        T: ControllerService,
                    > tonic::server::UnaryService<super::ScriptStatus>
                    for update_script_statusSvc<T> {
                        type Response = ();
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::ScriptStatus>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).update_script_status(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = update_script_statusSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/rule.ControllerService/update_device_desired" => {
                    #[allow(non_camel_case_types)]
                    struct update_device_desiredSvc<T: ControllerService>(pub Arc<T>);
                    impl<
                        T: ControllerService,
                    > tonic::server::UnaryService<super::UpdateDevice>
                    for update_device_desiredSvc<T> {
                        type Response = ();
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::UpdateDevice>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).update_device_desired(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = update_device_desiredSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                _ => {
                    Box::pin(async move {
                        Ok(
                            http::Response::builder()
                                .status(200)
                                .header("grpc-status", "12")
                                .header("content-type", "application/grpc")
                                .body(empty_body())
                                .unwrap(),
                        )
                    })
                }
            }
        }
    }
    impl<T: ControllerService> Clone for ControllerServiceServer<T> {
        fn clone(&self) -> Self {
            let inner = self.inner.clone();
            Self {
                inner,
                accept_compression_encodings: self.accept_compression_encodings,
                send_compression_encodings: self.send_compression_encodings,
            }
        }
    }
    impl<T: ControllerService> Clone for _Inner<T> {
        fn clone(&self) -> Self {
            Self(self.0.clone())
        }
    }
    impl<T: std::fmt::Debug> std::fmt::Debug for _Inner<T> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{:?}", self.0)
        }
    }
    impl<T: ControllerService> tonic::transport::NamedService
    for ControllerServiceServer<T> {
        const NAME: &'static str = "rule.ControllerService";
    }
}
