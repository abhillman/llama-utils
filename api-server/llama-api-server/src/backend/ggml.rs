use crate::{error, ModelInfo, CTX_SIZE};
use chat_prompts::{
    chat::{
        belle::BelleLlama2ChatPrompt,
        llama::{CodeLlamaInstructPrompt, Llama2ChatPrompt},
        mistral::{MistralInstructPrompt, MistralLitePrompt},
        openchat::OpenChatPrompt,
        BuildChatPrompt, ChatPrompt,
    },
    PromptTemplateType,
};
use endpoints::{
    chat::{
        ChatCompletionRequest, ChatCompletionResponse, ChatCompletionResponseChoice,
        ChatCompletionResponseMessage, ChatCompletionRole,
    },
    common::{FinishReason, Usage},
    completions::{CompletionChoice, CompletionObject, CompletionRequest},
    models::{ListModelsResponse, Model},
};
use hyper::{body::to_bytes, Body, Request, Response};
use std::time::SystemTime;

use std::time::Instant;

/// Lists models available
pub(crate) async fn models_handler(
    model_info: ModelInfo,
    template_ty: PromptTemplateType,
    created: u64,
) -> Result<Response<Body>, hyper::Error> {
    let model = Model {
        id: format!(
            "{name}:{template}",
            name = model_info.name,
            template = template_ty.to_string()
        ),
        created: created.clone(),
        object: String::from("model"),
        owned_by: String::from("Not specified"),
    };

    let list_models_response = ListModelsResponse {
        object: String::from("list"),
        data: vec![model],
    };

    // return response
    let result = Response::builder()
        .header("Access-Control-Allow-Origin", "*")
        .header("Access-Control-Allow-Methods", "*")
        .header("Access-Control-Allow-Headers", "*")
        .body(Body::from(
            serde_json::to_string(&list_models_response).unwrap(),
        ));
    match result {
        Ok(response) => Ok(response),
        Err(e) => error::internal_server_error(e.to_string()),
    }
}

pub(crate) async fn _embeddings_handler() -> Result<Response<Body>, hyper::Error> {
    println!("llama_embeddings_handler not implemented");
    error::not_implemented()
}

pub(crate) async fn completions_handler(
    mut req: Request<Body>,
) -> Result<Response<Body>, hyper::Error> {
    println!("[COMPLETION] New completion begins ...");

    // parse request
    let body_bytes = to_bytes(req.body_mut()).await?;
    let completion_request: CompletionRequest = serde_json::from_slice(&body_bytes).unwrap();

    let prompt = completion_request.prompt.join(" ");

    // ! todo: a temp solution of computing the number of tokens in prompt
    let prompt_tokens = prompt.split_whitespace().count() as u32;

    let buffer = match infer(prompt.trim()).await {
        Ok(buffer) => buffer,
        Err(e) => {
            return error::internal_server_error(e.to_string());
        }
    };

    // convert inference result to string
    let model_answer = String::from_utf8(buffer.clone()).unwrap();
    let answer = model_answer.trim();

    // ! todo: a temp solution of computing the number of tokens in answer
    let completion_tokens = answer.split_whitespace().count() as u32;

    println!("[COMPLETION] Bot answer: {}", answer);

    println!("[COMPLETION] New completion ends.");

    let completion_object = CompletionObject {
        id: uuid::Uuid::new_v4().to_string(),
        object: String::from("text_completion"),
        created: SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        model: completion_request.model.clone().unwrap_or_default(),
        choices: vec![CompletionChoice {
            index: 0,
            text: String::from(answer),
            finish_reason: FinishReason::stop,
            logprobs: None,
        }],
        usage: Usage {
            prompt_tokens,
            completion_tokens,
            total_tokens: prompt_tokens + completion_tokens,
        },
    };

    // return response
    let result = Response::builder()
        .header("Access-Control-Allow-Origin", "*")
        .header("Access-Control-Allow-Methods", "*")
        .header("Access-Control-Allow-Headers", "*")
        .body(Body::from(
            serde_json::to_string(&completion_object).unwrap(),
        ));
    match result {
        Ok(response) => Ok(response),
        Err(e) => error::internal_server_error(e.to_string()),
    }
}

/// Processes a chat-completion request and returns a chat-completion response with the answer from the model.
pub(crate) async fn chat_completions_handler(
    mut req: Request<Body>,
    template_ty: PromptTemplateType,
    log_prompts: bool,
) -> Result<Response<Body>, hyper::Error> {
    if req.method().eq(&hyper::http::Method::OPTIONS) {
        let result = Response::builder()
            .header("Access-Control-Allow-Origin", "*")
            .header("Access-Control-Allow-Methods", "*")
            .header("Access-Control-Allow-Headers", "*")
            .body(Body::empty());

        match result {
            Ok(response) => return Ok(response),
            Err(e) => {
                return error::internal_server_error(e.to_string());
            }
        }
    }

    fn create_prompt_template(template_ty: PromptTemplateType) -> ChatPrompt {
        match template_ty {
            PromptTemplateType::Llama2Chat => {
                ChatPrompt::Llama2ChatPrompt(Llama2ChatPrompt::default())
            }
            PromptTemplateType::MistralInstructV01 => {
                ChatPrompt::MistralInstructPrompt(MistralInstructPrompt::default())
            }
            PromptTemplateType::MistralLite => {
                ChatPrompt::MistralLitePrompt(MistralLitePrompt::default())
            }
            PromptTemplateType::OpenChat => ChatPrompt::OpenChatPrompt(OpenChatPrompt::default()),
            PromptTemplateType::CodeLlama => {
                ChatPrompt::CodeLlamaInstructPrompt(CodeLlamaInstructPrompt::default())
            }
            PromptTemplateType::BelleLlama2Chat => {
                ChatPrompt::BelleLlama2ChatPrompt(BelleLlama2ChatPrompt::default())
            }
            PromptTemplateType::VicunaChat => {
                ChatPrompt::VicunaChatPrompt(chat_prompts::chat::vicuna::VicunaChatPrompt::default())
            }
            PromptTemplateType::ChatML => {
                ChatPrompt::ChatMLPrompt(chat_prompts::chat::chatml::ChatMLPrompt::default())
            }
            PromptTemplateType::Baichuan2 => ChatPrompt::Baichuan2ChatPrompt(
                chat_prompts::chat::baichuan::Baichuan2ChatPrompt::default(),
            ),
            PromptTemplateType::WizardCoder => ChatPrompt::WizardCoderPrompt(
                chat_prompts::chat::wizard::WizardCoderPrompt::default(),
            ),
            PromptTemplateType::Zephyr => {
                ChatPrompt::ZephyrChatPrompt(chat_prompts::chat::zephyr::ZephyrChatPrompt::default())
            }
            PromptTemplateType::IntelNeural => {
                ChatPrompt::NeuralChatPrompt(chat_prompts::chat::intel::NeuralChatPrompt::default())
            }
            PromptTemplateType::DeepseekChat => ChatPrompt::DeepseekChatPrompt(
                chat_prompts::chat::deepseek::DeepseekChatPrompt::default(),
            ),
            PromptTemplateType::DeepseekCoder => ChatPrompt::DeepseekCoderPrompt(
                chat_prompts::chat::deepseek::DeepseekCoderPrompt::default(),
            ),
        }
    }
    let template = create_prompt_template(template_ty);

    println!("\n---------------- [LOG: Perf] ---------------------\n");

    // ! perf test: de-serialize request
    let start = Instant::now();

    // parse request
    let body_bytes = to_bytes(req.body_mut()).await?;
    let mut chat_request: ChatCompletionRequest = serde_json::from_slice(&body_bytes).unwrap();

    let duration = start.elapsed();
    println!(
        "[PERF] de-serialize request: {elapsed_in_ms} ms (or {elapsed_in_micros} micros)\n",
        elapsed_in_ms = duration.as_millis(),
        elapsed_in_micros = duration.as_micros()
    );

    // ! perf test: build prompt
    let start = Instant::now();

    // build prompt
    let prompt = match template.build(chat_request.messages.as_mut()) {
        Ok(prompt) => prompt,
        Err(e) => {
            return error::internal_server_error(e.to_string());
        }
    };

    let duration = start.elapsed();
    println!(
        "[PERF] build prompt: {elapsed_in_ms} ms (or {elapsed_in_micros} micros)\n",
        elapsed_in_ms = duration.as_millis(),
        elapsed_in_micros = duration.as_micros()
    );

    if log_prompts {
        println!("\n---------------- [LOG: PROMPT] ---------------------\n");
        println!("{}", &prompt);
        println!("\n----------------------------------------------------\n");
    }

    // ! todo: a temp solution of computing the number of tokens in prompt
    let prompt_tokens = prompt.split_whitespace().count() as u32;

    // ! perf test: inference
    let start = Instant::now();

    // run inference
    let buffer = match infer(prompt).await {
        Ok(buffer) => buffer,
        Err(e) => {
            return error::internal_server_error(e.to_string());
        }
    };

    let duration = start.elapsed();
    println!(
        "[PERF] the entire inference: {elapsed_in_ms} ms (or {elapsed_in_micros} micros)\n",
        elapsed_in_ms = duration.as_millis(),
        elapsed_in_micros = duration.as_micros()
    );

    // // ! perf test: post-process
    // let start = Instant::now();

    // // convert inference result to string
    // let output = String::from_utf8(buffer.clone()).unwrap();

    // let message = post_process(&output, template_ty);

    // let duration = start.elapsed();
    // println!(
    //     "[PERF] post-process inference result: {elapsed_in_ms} ms (or {elapsed_in_micros} micros)\n",
    //     elapsed_in_ms = duration.as_millis(),
    //     elapsed_in_micros = duration.as_micros()
    // );

    // ! perf test
    let message = String::from("This is a fake message for perf test.");

    // ! todo: a temp solution of computing the number of tokens in assistant_message
    let completion_tokens = message.split_whitespace().count() as u32;

    // ! perf test: build response
    let start = Instant::now();

    // create ChatCompletionResponse
    let chat_completion_obejct = ChatCompletionResponse {
        id: uuid::Uuid::new_v4().to_string(),
        object: String::from("chat.completion"),
        created: SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        model: chat_request.model.clone().unwrap_or_default(),
        choices: vec![ChatCompletionResponseChoice {
            index: 0,
            message: ChatCompletionResponseMessage {
                role: ChatCompletionRole::Assistant,
                content: message,
                function_call: None,
            },
            finish_reason: FinishReason::stop,
        }],
        usage: Usage {
            prompt_tokens,
            completion_tokens,
            total_tokens: prompt_tokens + completion_tokens,
        },
    };

    // return response
    let result = Response::builder()
        .header("Access-Control-Allow-Origin", "*")
        .header("Access-Control-Allow-Methods", "*")
        .header("Access-Control-Allow-Headers", "*")
        .body(Body::from(
            serde_json::to_string(&chat_completion_obejct).unwrap(),
        ));

    let duration = start.elapsed();
    println!(
        "[PERF] build response: {elapsed_in_ms} ms (or {elapsed_in_micros} micros)\n",
        elapsed_in_ms = duration.as_millis(),
        elapsed_in_micros = duration.as_micros()
    );

    println!("\n----------------------------------------------------\n");

    match result {
        Ok(response) => Ok(response),
        Err(e) => error::internal_server_error(e.to_string()),
    }
}

/// Runs inference on the model with the given name and returns the output.
pub(crate) async fn infer(prompt: impl AsRef<str>) -> std::result::Result<Vec<u8>, String> {
    let mut graph = crate::GRAPH.get().unwrap().lock().unwrap();

    // ! perf test: set input
    let start = Instant::now();

    let tensor_data = prompt.as_ref().as_bytes().to_vec();
    // println!("Read input tensor, size in bytes: {}", tensor_data.len());
    if graph
        .set_input(0, wasi_nn::TensorType::U8, &[1], &tensor_data)
        .is_err()
    {
        return Err(String::from("Fail to set input tensor"));
    };

    // // ! debug
    // drop(tensor_data);

    let duration = start.elapsed();
    println!(
        "[PERF] `set_input(tensor)` of inference: {elapsed_in_ms} ms (or {elapsed_in_micros} micros)",
        elapsed_in_ms = duration.as_millis(),
        elapsed_in_micros = duration.as_micros()
    );

    // ! perf test: compute
    let start = Instant::now();

    // execute the inference
    if graph.compute().is_err() {
        return Err(String::from("Fail to execute model inference"));
    }

    let duration = start.elapsed();
    println!(
        "[PERF] `compute` of inference: {elapsed_in_ms} ms (or {elapsed_in_micros} micros)",
        elapsed_in_ms = duration.as_millis(),
        elapsed_in_micros = duration.as_micros()
    );

    // ! perf test: get output
    let start = Instant::now();

    // Retrieve the output.
    let mut output_buffer = vec![0u8; *CTX_SIZE.get().unwrap()];
    let mut output_size = match graph.get_output(0, &mut output_buffer) {
        Ok(size) => size,
        Err(e) => {
            return Err(format!(
                "Fail to get output tensor: {msg}",
                msg = e.to_string()
            ))
        }
    };
    output_size = std::cmp::min(*CTX_SIZE.get().unwrap(), output_size);

    let duration = start.elapsed();
    println!(
        "[PERF] `get_output` of inference: {elapsed_in_ms} ms (or {elapsed_in_micros} micros)",
        elapsed_in_ms = duration.as_millis(),
        elapsed_in_micros = duration.as_micros()
    );

    Ok(output_buffer[..output_size].to_vec())

    // // ! perf test
    // Ok(vec![])
}

fn post_process(output: impl AsRef<str>, template_ty: PromptTemplateType) -> String {
    if template_ty == PromptTemplateType::Baichuan2 {
        output.as_ref().split('\n').collect::<Vec<_>>()[0]
            .trim()
            .to_owned()
    } else if template_ty == PromptTemplateType::OpenChat {
        output
            .as_ref()
            .trim_end_matches("<|end_of_turn|>")
            .trim()
            .to_owned()
    } else if template_ty == PromptTemplateType::ChatML {
        if output.as_ref().contains("<|im_end|>") {
            output.as_ref().replace("<|im_end|>", "").trim().to_owned()
        } else {
            output.as_ref().trim().to_owned()
        }
    } else if template_ty == PromptTemplateType::Zephyr
        || template_ty == PromptTemplateType::MistralLite
    {
        if output.as_ref().contains("</s>") {
            output.as_ref().trim_end_matches("</s>").trim().to_owned()
        } else {
            output.as_ref().trim().to_owned()
        }
    } else {
        output.as_ref().trim().to_owned()
    }
}

fn print(message: impl AsRef<str>) {
    println!("\n[ASSISTANT]:\n{}", message.as_ref().trim())
}
