use crate::v1::assistant::{
    AssistantFileObject, AssistantFileRequest, AssistantObject, AssistantRequest, DeletionStatus,
    ListAssistant, ListAssistantFile,
};
use crate::v1::audio::{
    AudioSpeechRequest, AudioSpeechResponse, AudioTranscriptionRequest, AudioTranscriptionResponse,
    AudioTranslationRequest, AudioTranslationResponse,
};
use crate::v1::chat_completion::{ChatCompletionRequest, ChatCompletionResponse};
use crate::v1::completion::{CompletionRequest, CompletionResponse};
use crate::v1::edit::{EditRequest, EditResponse};
use crate::v1::embedding::{EmbeddingRequest, EmbeddingResponse};
use crate::v1::error::APIError;
use crate::v1::file::{
    FileDeleteRequest, FileDeleteResponse, FileListResponse, FileRetrieveContentRequest,
    FileRetrieveContentResponse, FileRetrieveRequest, FileRetrieveResponse, FileUploadRequest,
    FileUploadResponse,
};
use crate::v1::fine_tuning::{
    CancelFineTuningJobRequest, CreateFineTuningJobRequest, FineTuningJobEvent,
    FineTuningJobObject, FineTuningPagination, ListFineTuningJobEventsRequest,
    RetrieveFineTuningJobRequest,
};
use crate::v1::image::{
    ImageEditRequest, ImageEditResponse, ImageGenerationRequest, ImageGenerationResponse,
    ImageVariationRequest, ImageVariationResponse,
};
use crate::v1::message::{
    CreateMessageRequest, ListMessage, ListMessageFile, MessageFileObject, MessageObject,
    ModifyMessageRequest,
};
use crate::v1::moderation::{CreateModerationRequest, CreateModerationResponse};
use crate::v1::run::{
    CreateRunRequest, CreateThreadAndRunRequest, ListRun, ListRunStep, ModifyRunRequest, RunObject,
    RunStepObject,
};
use crate::v1::thread::{CreateThreadRequest, ModifyThreadRequest, ThreadObject};

use ::futures::{stream, Stream, TryStreamExt};
use reqwest::header::HeaderMap;
use reqwest::RequestBuilder;
use std::collections::HashMap;
use std::fs::{create_dir_all, File};
use std::io::Write;
use std::path::Path;
use tokio::io::AsyncBufReadExt;
use tokio_util::io::StreamReader;

use super::chat_completion::ChatChunkResponse;

const API_URL_V1: &str = "http://0.0.0.0:1234/v1";

#[derive(Clone)]
pub struct Client {
    pub api_endpoint: String,
    pub api_key: String,
    pub organization: Option<String>,
    pub proxy: Option<String>,
    http_client: reqwest::Client,
}

impl Client {
    pub fn new(api_key: String) -> Self {
        let endpoint = std::env::var("API_URL_V1").unwrap_or_else(|_| API_URL_V1.to_owned());
        Self::new_with_endpoint(endpoint, api_key)
    }

    pub fn new_with_endpoint(api_endpoint: String, api_key: String) -> Self {
        Self {
            api_endpoint,
            api_key,
            organization: None,
            proxy: None,
            http_client: reqwest::Client::new(),
        }
    }

    pub fn new_with_organization(api_key: String, organization: String) -> Self {
        let endpoint = std::env::var("API_URL_V1").unwrap_or_else(|_| API_URL_V1.to_owned());
        let mut client = Self::new_with_endpoint(endpoint, api_key);
        client.organization = organization.into();
        return client;
    }

    pub fn new_with_proxy(api_key: String, proxy: String) -> Self {
        let api_endpoint = std::env::var("API_URL_V1").unwrap_or_else(|_| API_URL_V1.to_owned());

        Self {
            api_endpoint,
            api_key,
            organization: None,
            proxy: Some(proxy.clone()),
            http_client: reqwest::Client::builder()
                .proxy(reqwest::Proxy::all(proxy).expect("proxy format incorrect"))
                .build()
                .unwrap(),
        }
    }

    pub fn build_request(&self, req_builder: RequestBuilder, is_beta: bool) -> RequestBuilder {
        let mut builder = req_builder
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", self.api_key));
        if let Some(organization) = &self.organization {
            builder = builder.header("tupleleapai-organization", organization);
        }
        if is_beta {
            builder = builder.header("tupleleapai-Beta", "assistants=v1");
        }
        builder
    }

    pub fn build_request_stream(
        &self,
        req_builder: RequestBuilder,
        is_beta: bool,
    ) -> RequestBuilder {
        let builder = self
            .build_request(req_builder, is_beta)
            .header("Accept", "text/event-stream");
        builder
    }

    pub async fn post<T: serde::ser::Serialize>(
        &self,
        path: &str,
        params: &T,
    ) -> Result<reqwest::Response, APIError> {
        let url = format!(
            "{api_endpoint}{path}",
            api_endpoint = self.api_endpoint,
            path = path
        );

        let request = self.build_request(self.http_client.post(url), Self::is_beta(path));
        let res = request.json(params).send().await;
        match res {
            Ok(res) => res.error_for_status().map_err(|e| APIError {
                message: format!("{}", e),
            }),
            Err(e) => Err(APIError {
                message: format!("{}", e),
            }),
        }
    }

    pub async fn post_stream<T: serde::ser::Serialize>(
        &self,
        path: &str,
        params: &T,
    ) -> Result<reqwest::Response, APIError> {
        let url = format!(
            "{api_endpoint}{path}",
            api_endpoint = self.api_endpoint,
            path = path
        );

        let request = self.build_request_stream(self.http_client.post(url), Self::is_beta(path));
        let res = request.json(params).send().await;
        match res {
            Ok(res) => res.error_for_status().map_err(|e| APIError {
                message: format!("{}", e),
            }),
            Err(e) => Err(APIError {
                message: format!("{}", e),
            }),
        }
    }

    pub async fn get(&self, path: &str) -> Result<reqwest::Response, APIError> {
        let url = format!(
            "{api_endpoint}{path}",
            api_endpoint = self.api_endpoint,
            path = path
        );
        let request = self.build_request(self.http_client.get(url), Self::is_beta(path));
        let res = request.send().await;
        match res {
            Ok(res) => res.error_for_status().map_err(|e| APIError {
                message: format!("{}", e),
            }),
            Err(e) => Err(self.new_error(e)),
        }
    }

    pub async fn delete(&self, path: &str) -> Result<reqwest::Response, APIError> {
        let url = format!(
            "{api_endpoint}{path}",
            api_endpoint = self.api_endpoint,
            path = path
        );
        let request = self.build_request(self.http_client.delete(url), Self::is_beta(path));
        let res = request.send().await;
        match res {
            Ok(res) => res.error_for_status().map_err(|e| APIError {
                message: format!("{}", e),
            }),
            Err(e) => Err(self.new_error(e)),
        }
    }

    pub async fn completion(&self, req: CompletionRequest) -> Result<CompletionResponse, APIError> {
        let res = self.post("/completions", &req).await?;
        let headers = res.headers().clone();
        let r = res.json::<CompletionResponse>().await;
        match r {
            Ok(mut r) => {
                r.headers = Some(Self::convert_to_map(headers));
                Ok(r)
            }
            Err(e) => Err(self.new_error(e)),
        }
    }

    fn convert_to_map(headers: HeaderMap) -> HashMap<String, String> {
        headers
            .into_iter()
            .map(|(name, value)| {
                (
                    name.unwrap().to_string(),
                    value.to_str().unwrap().to_owned(),
                )
            })
            .collect()
    }

    pub async fn edit(&self, req: EditRequest) -> Result<EditResponse, APIError> {
        let res = self.post("/edits", &req).await?;
        let headers = res.headers().clone();
        let r = res.json::<EditResponse>().await;
        match r {
            Ok(mut r) => {
                r.headers = Some(Self::convert_to_map(headers));
                Ok(r)
            }
            Err(e) => Err(self.new_error(e)),
        }
    }

    pub async fn image_generation(
        &self,
        req: ImageGenerationRequest,
    ) -> Result<ImageGenerationResponse, APIError> {
        let res = self.post("/images/generations", &req).await?;
        let headers = res.headers().clone();
        let r = res.json::<ImageGenerationResponse>().await;
        match r {
            Ok(mut r) => {
                r.headers = Some(Self::convert_to_map(headers));
                Ok(r)
            }
            Err(e) => Err(self.new_error(e)),
        }
    }

    pub async fn image_edit(&self, req: ImageEditRequest) -> Result<ImageEditResponse, APIError> {
        let res = self.post("/images/edits", &req).await?;
        let headers = res.headers().clone();
        let r = res.json::<ImageEditResponse>().await;
        match r {
            Ok(mut r) => {
                r.headers = Some(Self::convert_to_map(headers));
                Ok(r)
            }
            Err(e) => Err(self.new_error(e)),
        }
    }

    pub async fn image_variation(
        &self,
        req: ImageVariationRequest,
    ) -> Result<ImageVariationResponse, APIError> {
        let res = self.post("/images/variations", &req).await?;
        let headers = res.headers().clone();
        let r = res.json::<ImageVariationResponse>().await;
        match r {
            Ok(mut r) => {
                r.headers = Some(Self::convert_to_map(headers));
                Ok(r)
            }
            Err(e) => Err(self.new_error(e)),
        }
    }

    pub async fn embedding(&self, req: EmbeddingRequest) -> Result<EmbeddingResponse, APIError> {
        let res = self.post("/embeddings", &req).await?;
        let headers = res.headers().clone();
        let r = res.json::<EmbeddingResponse>().await;
        match r {
            Ok(mut r) => {
                r.headers = Some(Self::convert_to_map(headers));
                Ok(r)
            }
            Err(e) => Err(self.new_error(e)),
        }
    }

    pub async fn file_list(&self) -> Result<FileListResponse, APIError> {
        let res = self.get("/files").await?;
        let headers = res.headers().clone();
        let r = res.json::<FileListResponse>().await;
        match r {
            Ok(mut r) => {
                r.headers = Some(Self::convert_to_map(headers));
                Ok(r)
            }
            Err(e) => Err(self.new_error(e)),
        }
    }

    pub async fn file_upload(
        &self,
        req: FileUploadRequest,
    ) -> Result<FileUploadResponse, APIError> {
        let res = self.post("/files", &req).await?;
        let headers = res.headers().clone();
        let r = res.json::<FileUploadResponse>().await;
        match r {
            Ok(mut r) => {
                r.headers = Some(Self::convert_to_map(headers));
                Ok(r)
            }
            Err(e) => Err(self.new_error(e)),
        }
    }

    pub async fn file_delete(
        &self,
        req: FileDeleteRequest,
    ) -> Result<FileDeleteResponse, APIError> {
        let res = self
            .delete(&format!("{}/{}", "/files", req.file_id))
            .await?;
        let headers = res.headers().clone();
        let r = res.json::<FileDeleteResponse>().await;
        match r {
            Ok(mut r) => {
                r.headers = Some(Self::convert_to_map(headers));
                Ok(r)
            }
            Err(e) => Err(self.new_error(e)),
        }
    }

    pub async fn file_retrieve(
        &self,
        req: FileRetrieveRequest,
    ) -> Result<FileRetrieveResponse, APIError> {
        let res = self.get(&format!("{}/{}", "/files", req.file_id)).await?;
        let headers = res.headers().clone();
        let r = res.json::<FileRetrieveResponse>().await;
        match r {
            Ok(mut r) => {
                r.headers = Some(Self::convert_to_map(headers));
                Ok(r)
            }
            Err(e) => Err(self.new_error(e)),
        }
    }

    pub async fn file_retrieve_content(
        &self,
        req: FileRetrieveContentRequest,
    ) -> Result<FileRetrieveContentResponse, APIError> {
        let res = self
            .get(&format!("{}/{}/content", "/files", req.file_id))
            .await?;
        let headers = res.headers().clone();
        let r = res.json::<FileRetrieveContentResponse>().await;
        match r {
            Ok(mut r) => {
                r.headers = Some(Self::convert_to_map(headers));
                Ok(r)
            }
            Err(e) => Err(self.new_error(e)),
        }
    }

    pub async fn chat_completion(
        &self,
        req: ChatCompletionRequest,
    ) -> Result<ChatCompletionResponse, APIError> {
        let res = self.post("/chat/completions", &req).await?;
        let headers = res.headers().clone();
        let r = res.json::<ChatCompletionResponse>().await;
        match r {
            Ok(mut r) => {
                r.headers = Some(Self::convert_to_map(headers));
                Ok(r)
            }
            Err(e) => Err(self.new_error(e)),
        }
    }

    // fn response_to_async_read(resp: reqwest::Response) -> impl tokio::io::AsyncRead {
    //     let stream = resp.bytes_stream().map_err(std::io::Error::other);
    //     tokio_util::io::StreamReader::new(stream)
    // }

    fn read_chunk(line: String) -> Result<ChatChunkResponse, APIError> {
        let ser_data: &str = line.trim();
        if ser_data.is_empty() || ser_data.starts_with("data:") {
            match ser_data.splitn(2, "data:").last() {
                Some(msg) => {
                    let t3: Result<ChatChunkResponse, APIError> = match serde_json::from_str(msg) {
                        Ok(chunk) => Ok(chunk),
                        Err(e) => Err(APIError {
                            message: e.to_string(),
                        }),
                    };
                    return t3;
                }
                None => Err(APIError {
                    message: "invalid string, ignoring it".into(),
                }),
            }
        } else {
            Err(APIError {
                message: "invalid string, ignoring it".into(),
            })
        }
    }

    pub async fn chat_completion_stream(
        &self,
        req: ChatCompletionRequest,
    ) -> Result<impl Stream<Item = ChatChunkResponse>, APIError> {
        let res = self
            .post_stream("/chat/completions", &(req.stream(true)))
            .await?;
        if !res.status().is_success() {
            return Err(APIError {
                message: res.text().await.unwrap_or_else(|e| e.to_string()),
            });
        }
        // Obtain a byte stream from the response.
        let bytes_stream = res.bytes_stream().map_err(std::io::Error::other);
        //Convert a [Stream] of byte chunks into an [AsyncRead].
        let reader = StreamReader::new(bytes_stream);
        // This creates a stream with closure returning a future.
        let stream = stream::unfold(reader, |mut reader| async move {
            loop {
                let mut line_data = String::new();
                // Read line from the underlying stream.
                let line_result: Result<usize, std::io::Error> =
                    reader.read_line(&mut line_data).await;

                if line_result.is_err() {
                    println!(
                        "Error observed while erading from response {:?}",
                        line_result.err()
                    );
                    return None;
                } else {
                    if line_result.unwrap() == 0 {
                        // Nothing to read, end the stream.
                        return None;
                    } else {
                        let msg = line_data;
                        // parse the data and return a ChatChunkResponse.
                        let read_result = Self::read_chunk(msg.clone());
                        if read_result.is_ok() {
                            // println!("Read line {}", msg);
                            // Create a new object due to ownership issue, also the clone method is not implemented in the tokio lib
                            let new_reader = StreamReader::new(reader.into_inner());
                            return Some((read_result.unwrap(), new_reader));
                        } else {
                            // Do nothing skip and read the next line.
                            // println!("Invalid data observed while trying to read the chunk, read the next chunk")
                        }
                    }
                }
            }
        });
        return Ok(stream);
    }

    pub async fn audio_transcription(
        &self,
        req: AudioTranscriptionRequest,
    ) -> Result<AudioTranscriptionResponse, APIError> {
        let res = self.post("/audio/transcriptions", &req).await?;
        let headers = res.headers().clone();
        let r = res.json::<AudioTranscriptionResponse>().await;
        match r {
            Ok(mut r) => {
                r.headers = Some(Self::convert_to_map(headers));
                Ok(r)
            }
            Err(e) => Err(self.new_error(e)),
        }
    }

    pub async fn audio_translation(
        &self,
        req: AudioTranslationRequest,
    ) -> Result<AudioTranslationResponse, APIError> {
        let res = self.post("/audio/translations", &req).await?;
        let headers = res.headers().clone();
        let r = res.json::<AudioTranslationResponse>().await;
        match r {
            Ok(mut r) => {
                r.headers = Some(Self::convert_to_map(headers));
                Ok(r)
            }
            Err(e) => Err(self.new_error(e)),
        }
    }

    pub async fn audio_speech(
        &self,
        req: AudioSpeechRequest,
    ) -> Result<AudioSpeechResponse, APIError> {
        let res = self.post("/audio/speech", &req).await?;
        let headers = res.headers().clone();
        let bytes = res.bytes().await.unwrap();
        let path = req.output.as_str();
        let path = Path::new(path);
        if let Some(parent) = path.parent() {
            match create_dir_all(parent) {
                Ok(_) => {}
                Err(e) => {
                    return Err(APIError {
                        message: e.to_string(),
                    })
                }
            }
        }
        match File::create(path) {
            Ok(mut file) => match file.write_all(&bytes) {
                Ok(_) => {}
                Err(e) => {
                    return Err(APIError {
                        message: e.to_string(),
                    })
                }
            },
            Err(e) => {
                return Err(APIError {
                    message: e.to_string(),
                })
            }
        }
        Ok(AudioSpeechResponse {
            result: true,
            headers: Some(Self::convert_to_map(headers)),
        })
    }

    pub async fn create_fine_tuning_job(
        &self,
        req: CreateFineTuningJobRequest,
    ) -> Result<FineTuningJobObject, APIError> {
        let res = self.post("/fine_tuning/jobs", &req).await?;
        let headers = res.headers().clone();
        let r = res.json::<FineTuningJobObject>().await;
        match r {
            Ok(mut r) => {
                r.headers = Some(Self::convert_to_map(headers));
                Ok(r)
            }
            Err(e) => Err(self.new_error(e)),
        }
    }

    pub async fn list_fine_tuning_jobs(
        &self,
    ) -> Result<FineTuningPagination<FineTuningJobObject>, APIError> {
        let res = self.get("/fine_tuning/jobs").await?;
        let headers = res.headers().clone();
        let r = res
            .json::<FineTuningPagination<FineTuningJobObject>>()
            .await;
        match r {
            Ok(mut r) => {
                r.headers = Some(Self::convert_to_map(headers));
                Ok(r)
            }
            Err(e) => Err(self.new_error(e)),
        }
    }

    pub async fn list_fine_tuning_job_events(
        &self,
        req: ListFineTuningJobEventsRequest,
    ) -> Result<FineTuningPagination<FineTuningJobEvent>, APIError> {
        let res = self
            .get(&format!(
                "/fine_tuning/jobs/{}/events",
                req.fine_tuning_job_id
            ))
            .await?;
        let headers = res.headers().clone();
        let r = res.json::<FineTuningPagination<FineTuningJobEvent>>().await;
        match r {
            Ok(mut r) => {
                r.headers = Some(Self::convert_to_map(headers));
                Ok(r)
            }
            Err(e) => Err(self.new_error(e)),
        }
    }

    pub async fn retrieve_fine_tuning_job(
        &self,
        req: RetrieveFineTuningJobRequest,
    ) -> Result<FineTuningJobObject, APIError> {
        let res = self
            .get(&format!("/fine_tuning/jobs/{}", req.fine_tuning_job_id))
            .await?;
        let headers = res.headers().clone();
        let r = res.json::<FineTuningJobObject>().await;
        match r {
            Ok(mut r) => {
                r.headers = Some(Self::convert_to_map(headers));
                Ok(r)
            }
            Err(e) => Err(self.new_error(e)),
        }
    }

    pub async fn cancel_fine_tuning_job(
        &self,
        req: CancelFineTuningJobRequest,
    ) -> Result<FineTuningJobObject, APIError> {
        let res = self
            .post(
                &format!("/fine_tuning/jobs/{}/cancel", req.fine_tuning_job_id),
                &req,
            )
            .await?;
        let headers = res.headers().clone();
        let r = res.json::<FineTuningJobObject>().await;
        match r {
            Ok(mut r) => {
                r.headers = Some(Self::convert_to_map(headers));
                Ok(r)
            }
            Err(e) => Err(self.new_error(e)),
        }
    }

    pub async fn create_moderation(
        &self,
        req: CreateModerationRequest,
    ) -> Result<CreateModerationResponse, APIError> {
        let res = self.post("/moderations", &req).await?;
        let headers = res.headers().clone();
        let r = res.json::<CreateModerationResponse>().await;
        match r {
            Ok(mut r) => {
                r.headers = Some(Self::convert_to_map(headers));
                Ok(r)
            }
            Err(e) => Err(self.new_error(e)),
        }
    }

    pub async fn create_assistant(
        &self,
        req: AssistantRequest,
    ) -> Result<AssistantObject, APIError> {
        let res = self.post("/assistants", &req).await?;
        let headers = res.headers().clone();
        let r = res.json::<AssistantObject>().await;
        match r {
            Ok(mut r) => {
                r.headers = Some(Self::convert_to_map(headers));
                Ok(r)
            }
            Err(e) => Err(self.new_error(e)),
        }
    }

    pub async fn retrieve_assistant(
        &self,
        assistant_id: String,
    ) -> Result<AssistantObject, APIError> {
        let res = self.get(&format!("/assistants/{}", assistant_id)).await?;
        let headers = res.headers().clone();
        let r = res.json::<AssistantObject>().await;
        match r {
            Ok(mut r) => {
                r.headers = Some(Self::convert_to_map(headers));
                Ok(r)
            }
            Err(e) => Err(self.new_error(e)),
        }
    }

    pub async fn modify_assistant(
        &self,
        assistant_id: String,
        req: AssistantRequest,
    ) -> Result<AssistantObject, APIError> {
        let res = self
            .post(&format!("/assistants/{}", assistant_id), &req)
            .await?;
        let headers = res.headers().clone();
        let r = res.json::<AssistantObject>().await;
        match r {
            Ok(mut r) => {
                r.headers = Some(Self::convert_to_map(headers));
                Ok(r)
            }
            Err(e) => Err(self.new_error(e)),
        }
    }

    pub async fn delete_assistant(&self, assistant_id: String) -> Result<DeletionStatus, APIError> {
        let res = self
            .delete(&format!("/assistants/{}", assistant_id))
            .await?;
        let headers = res.headers().clone();
        let r = res.json::<DeletionStatus>().await;
        match r {
            Ok(mut r) => {
                r.headers = Some(Self::convert_to_map(headers));
                Ok(r)
            }
            Err(e) => Err(self.new_error(e)),
        }
    }

    pub async fn list_assistant(
        &self,
        limit: Option<i64>,
        order: Option<String>,
        after: Option<String>,
        before: Option<String>,
    ) -> Result<ListAssistant, APIError> {
        let mut url = "/assistants".to_owned();
        url = Self::query_params(limit, order, after, before, url);
        let res = self.get(&url).await?;
        let headers = res.headers().clone();
        let r = res.json::<ListAssistant>().await;
        match r {
            Ok(mut r) => {
                r.headers = Some(Self::convert_to_map(headers));
                Ok(r)
            }
            Err(e) => Err(self.new_error(e)),
        }
    }

    pub async fn create_assistant_file(
        &self,
        assistant_id: String,
        req: AssistantFileRequest,
    ) -> Result<AssistantFileObject, APIError> {
        let res = self
            .post(&format!("/assistants/{}/files", assistant_id), &req)
            .await?;
        let headers = res.headers().clone();
        let r = res.json::<AssistantFileObject>().await;
        match r {
            Ok(mut r) => {
                r.headers = Some(Self::convert_to_map(headers));
                Ok(r)
            }
            Err(e) => Err(self.new_error(e)),
        }
    }

    pub async fn retrieve_assistant_file(
        &self,
        assistant_id: String,
        file_id: String,
    ) -> Result<AssistantFileObject, APIError> {
        let res = self
            .get(&format!("/assistants/{}/files/{}", assistant_id, file_id))
            .await?;
        let headers = res.headers().clone();
        let r = res.json::<AssistantFileObject>().await;
        match r {
            Ok(mut r) => {
                r.headers = Some(Self::convert_to_map(headers));
                Ok(r)
            }
            Err(e) => Err(self.new_error(e)),
        }
    }

    pub async fn delete_assistant_file(
        &self,
        assistant_id: String,
        file_id: String,
    ) -> Result<DeletionStatus, APIError> {
        let res = self
            .delete(&format!("/assistants/{}/files/{}", assistant_id, file_id))
            .await?;
        let headers = res.headers().clone();
        let r = res.json::<DeletionStatus>().await;
        match r {
            Ok(mut r) => {
                r.headers = Some(Self::convert_to_map(headers));
                Ok(r)
            }
            Err(e) => Err(self.new_error(e)),
        }
    }

    pub async fn list_assistant_file(
        &self,
        assistant_id: String,
        limit: Option<i64>,
        order: Option<String>,
        after: Option<String>,
        before: Option<String>,
    ) -> Result<ListAssistantFile, APIError> {
        let mut url = format!("/assistants/{}/files", assistant_id);
        url = Self::query_params(limit, order, after, before, url);
        let res = self.get(&url).await?;
        let headers = res.headers().clone();
        let r = res.json::<ListAssistantFile>().await;
        match r {
            Ok(mut r) => {
                r.headers = Some(Self::convert_to_map(headers));
                Ok(r)
            }
            Err(e) => Err(self.new_error(e)),
        }
    }

    pub async fn create_thread(&self, req: CreateThreadRequest) -> Result<ThreadObject, APIError> {
        let res = self.post("/threads", &req).await?;
        let headers = res.headers().clone();
        let r = res.json::<ThreadObject>().await;
        match r {
            Ok(mut r) => {
                r.headers = Some(Self::convert_to_map(headers));
                Ok(r)
            }
            Err(e) => Err(self.new_error(e)),
        }
    }

    pub async fn retrieve_thread(&self, thread_id: String) -> Result<ThreadObject, APIError> {
        let res = self.get(&format!("/threads/{}", thread_id)).await?;
        let headers = res.headers().clone();
        let r = res.json::<ThreadObject>().await;
        match r {
            Ok(mut r) => {
                r.headers = Some(Self::convert_to_map(headers));
                Ok(r)
            }
            Err(e) => Err(self.new_error(e)),
        }
    }

    pub async fn modify_thread(
        &self,
        thread_id: String,
        req: ModifyThreadRequest,
    ) -> Result<ThreadObject, APIError> {
        let res = self.post(&format!("/threads/{}", thread_id), &req).await?;
        let headers = res.headers().clone();
        let r = res.json::<ThreadObject>().await;
        match r {
            Ok(mut r) => {
                r.headers = Some(Self::convert_to_map(headers));
                Ok(r)
            }
            Err(e) => Err(self.new_error(e)),
        }
    }

    pub async fn delete_thread(&self, thread_id: String) -> Result<DeletionStatus, APIError> {
        let res = self.delete(&format!("/threads/{}", thread_id)).await?;
        let headers = res.headers().clone();
        let r = res.json::<DeletionStatus>().await;
        match r {
            Ok(mut r) => {
                r.headers = Some(Self::convert_to_map(headers));
                Ok(r)
            }
            Err(e) => Err(self.new_error(e)),
        }
    }

    pub async fn create_message(
        &self,
        thread_id: String,
        req: CreateMessageRequest,
    ) -> Result<MessageObject, APIError> {
        let res = self
            .post(&format!("/threads/{}/messages", thread_id), &req)
            .await?;
        let headers = res.headers().clone();
        let r = res.json::<MessageObject>().await;
        match r {
            Ok(mut r) => {
                r.headers = Some(Self::convert_to_map(headers));
                Ok(r)
            }
            Err(e) => Err(self.new_error(e)),
        }
    }

    pub async fn retrieve_message(
        &self,
        thread_id: String,
        message_id: String,
    ) -> Result<MessageObject, APIError> {
        let res = self
            .get(&format!("/threads/{}/messages/{}", thread_id, message_id))
            .await?;
        let headers = res.headers().clone();
        let r = res.json::<MessageObject>().await;
        match r {
            Ok(mut r) => {
                r.headers = Some(Self::convert_to_map(headers));
                Ok(r)
            }
            Err(e) => Err(self.new_error(e)),
        }
    }

    pub async fn modify_message(
        &self,
        thread_id: String,
        message_id: String,
        req: ModifyMessageRequest,
    ) -> Result<MessageObject, APIError> {
        let res = self
            .post(
                &format!("/threads/{}/messages/{}", thread_id, message_id),
                &req,
            )
            .await?;
        let headers = res.headers().clone();
        let r = res.json::<MessageObject>().await;
        match r {
            Ok(mut r) => {
                r.headers = Some(Self::convert_to_map(headers));
                Ok(r)
            }
            Err(e) => Err(self.new_error(e)),
        }
    }

    pub async fn list_messages(&self, thread_id: String) -> Result<ListMessage, APIError> {
        let res = self
            .get(&format!("/threads/{}/messages", thread_id))
            .await?;
        let headers = res.headers().clone();
        let r = res.json::<ListMessage>().await;
        match r {
            Ok(mut r) => {
                r.headers = Some(Self::convert_to_map(headers));
                Ok(r)
            }
            Err(e) => Err(self.new_error(e)),
        }
    }

    pub async fn retrieve_message_file(
        &self,
        thread_id: String,
        message_id: String,
        file_id: String,
    ) -> Result<MessageFileObject, APIError> {
        let res = self
            .get(&format!(
                "/threads/{}/messages/{}/files/{}",
                thread_id, message_id, file_id
            ))
            .await?;
        let headers = res.headers().clone();
        let r = res.json::<MessageFileObject>().await;
        match r {
            Ok(mut r) => {
                r.headers = Some(Self::convert_to_map(headers));
                Ok(r)
            }
            Err(e) => Err(self.new_error(e)),
        }
    }

    pub async fn list_message_file(
        &self,
        thread_id: String,
        message_id: String,
        limit: Option<i64>,
        order: Option<String>,
        after: Option<String>,
        before: Option<String>,
    ) -> Result<ListMessageFile, APIError> {
        let mut url = format!("/threads/{}/messages/{}/files", thread_id, message_id);
        url = Self::query_params(limit, order, after, before, url);
        let res = self.get(&url).await?;
        let headers = res.headers().clone();
        let r = res.json::<ListMessageFile>().await;
        match r {
            Ok(mut r) => {
                r.headers = Some(Self::convert_to_map(headers));
                Ok(r)
            }
            Err(e) => Err(self.new_error(e)),
        }
    }

    pub async fn create_run(
        &self,
        thread_id: String,
        req: CreateRunRequest,
    ) -> Result<RunObject, APIError> {
        let res = self
            .post(&format!("/threads/{}/runs", thread_id), &req)
            .await?;
        let headers = res.headers().clone();
        let r = res.json::<RunObject>().await;
        match r {
            Ok(mut r) => {
                r.headers = Some(Self::convert_to_map(headers));
                Ok(r)
            }
            Err(e) => Err(self.new_error(e)),
        }
    }

    pub async fn retrieve_run(
        &self,
        thread_id: String,
        run_id: String,
    ) -> Result<RunObject, APIError> {
        let res = self
            .get(&format!("/threads/{}/runs/{}", thread_id, run_id))
            .await?;
        let headers = res.headers().clone();
        let r = res.json::<RunObject>().await;
        match r {
            Ok(mut r) => {
                r.headers = Some(Self::convert_to_map(headers));
                Ok(r)
            }
            Err(e) => Err(self.new_error(e)),
        }
    }

    pub async fn modify_run(
        &self,
        thread_id: String,
        run_id: String,
        req: ModifyRunRequest,
    ) -> Result<RunObject, APIError> {
        let res = self
            .post(&format!("/threads/{}/runs/{}", thread_id, run_id), &req)
            .await?;
        let headers = res.headers().clone();
        let r = res.json::<RunObject>().await;
        match r {
            Ok(mut r) => {
                r.headers = Some(Self::convert_to_map(headers));
                Ok(r)
            }
            Err(e) => Err(self.new_error(e)),
        }
    }

    pub async fn list_run(
        &self,
        thread_id: String,
        limit: Option<i64>,
        order: Option<String>,
        after: Option<String>,
        before: Option<String>,
    ) -> Result<ListRun, APIError> {
        let mut url = format!("/threads/{}/runs", thread_id);
        url = Self::query_params(limit, order, after, before, url);
        let res = self.get(&url).await?;
        let headers = res.headers().clone();
        let r = res.json::<ListRun>().await;
        match r {
            Ok(mut r) => {
                r.headers = Some(Self::convert_to_map(headers));
                Ok(r)
            }
            Err(e) => Err(self.new_error(e)),
        }
    }

    pub async fn cancel_run(
        &self,
        thread_id: String,
        run_id: String,
    ) -> Result<RunObject, APIError> {
        let empty_req = ModifyRunRequest::new();
        let res = self
            .post(
                &format!("/threads/{}/runs/{}/cancel", thread_id, run_id),
                &empty_req,
            )
            .await?;
        let headers = res.headers().clone();
        let r = res.json::<RunObject>().await;
        match r {
            Ok(mut r) => {
                r.headers = Some(Self::convert_to_map(headers));
                Ok(r)
            }
            Err(e) => Err(self.new_error(e)),
        }
    }

    pub async fn create_thread_and_run(
        &self,
        req: CreateThreadAndRunRequest,
    ) -> Result<RunObject, APIError> {
        let res = self.post("/threads/runs", &req).await?;
        let headers = res.headers().clone();
        let r = res.json::<RunObject>().await;
        match r {
            Ok(mut r) => {
                r.headers = Some(Self::convert_to_map(headers));
                Ok(r)
            }
            Err(e) => Err(self.new_error(e)),
        }
    }

    pub async fn retrieve_run_step(
        &self,
        thread_id: String,
        run_id: String,
        step_id: String,
    ) -> Result<RunStepObject, APIError> {
        let res = self
            .get(&format!(
                "/threads/{}/runs/{}/steps/{}",
                thread_id, run_id, step_id
            ))
            .await?;
        let headers = res.headers().clone();
        let r = res.json::<RunStepObject>().await;
        match r {
            Ok(mut r) => {
                r.headers = Some(Self::convert_to_map(headers));
                Ok(r)
            }
            Err(e) => Err(self.new_error(e)),
        }
    }

    pub async fn list_run_step(
        &self,
        thread_id: String,
        run_id: String,
        limit: Option<i64>,
        order: Option<String>,
        after: Option<String>,
        before: Option<String>,
    ) -> Result<ListRunStep, APIError> {
        let mut url = format!("/threads/{}/runs/{}/steps", thread_id, run_id);
        url = Self::query_params(limit, order, after, before, url);
        let res = self.get(&url).await?;
        let headers = res.headers().clone();
        let r = res.json::<ListRunStep>().await;
        match r {
            Ok(mut r) => {
                r.headers = Some(Self::convert_to_map(headers));
                Ok(r)
            }
            Err(e) => Err(self.new_error(e)),
        }
    }

    fn new_error(&self, err: reqwest::Error) -> APIError {
        APIError {
            message: err.to_string(),
        }
    }

    fn is_beta(path: &str) -> bool {
        path.starts_with("/assistants") || path.starts_with("/threads")
    }

    fn query_params(
        limit: Option<i64>,
        order: Option<String>,
        after: Option<String>,
        before: Option<String>,
        mut url: String,
    ) -> String {
        let mut params = vec![];
        if let Some(limit) = limit {
            params.push(format!("limit={}", limit));
        }
        if let Some(order) = order {
            params.push(format!("order={}", order));
        }
        if let Some(after) = after {
            params.push(format!("after={}", after));
        }
        if let Some(before) = before {
            params.push(format!("before={}", before));
        }
        if !params.is_empty() {
            url = format!("{}?{}", url, params.join("&"));
        }
        url
    }
}
