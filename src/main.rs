use serde::{Deserialize, Serialize};
use serde_json::Value;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::notification::Notification;
use tower_lsp::lsp_types::ServerInfo;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

const LEGEND_TYPE: &[SemanticTokenType] = &[
    SemanticTokenType::FUNCTION,
    SemanticTokenType::VARIABLE,
    SemanticTokenType::STRING,
    SemanticTokenType::COMMENT,
    SemanticTokenType::NUMBER,
    SemanticTokenType::KEYWORD,
    SemanticTokenType::OPERATOR,
    SemanticTokenType::PARAMETER,
];

/// Definition of the server
#[derive(Debug)]
struct Backend {
    client: Client,
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            server_info: Some(ServerInfo {
                name: "Generic language server".to_string(),
                version: Some("0.0.1".to_string()),
            }),

            offset_encoding: None,

            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    // TextDocumentSyncKind::NONE
                    TextDocumentSyncKind::INCREMENTAL,
                    // TextDocumentSyncKind::FULL,
                )),
                inlay_hint_provider: Some(OneOf::Left(true)),
                completion_provider: Some(CompletionOptions {
                    resolve_provider: Some(false),
                    trigger_characters: Some(vec![".".to_string(), " ".to_string()]),
                    work_done_progress_options: Default::default(),
                    all_commit_characters: None,
                    completion_item: None,
                }),
                execute_command_provider: Some(ExecuteCommandOptions {
                    // TODO - how this works?
                    commands: vec!["dummy.do_something".to_string()],
                    work_done_progress_options: Default::default(),
                }),

                workspace: Some(WorkspaceServerCapabilities {
                    workspace_folders: Some(WorkspaceFoldersServerCapabilities {
                        supported: Some(true),
                        change_notifications: Some(OneOf::Left(true)),
                    }),
                    file_operations: None,
                }),

                semantic_tokens_provider: Some(
                    SemanticTokensServerCapabilities::SemanticTokensRegistrationOptions(
                        SemanticTokensRegistrationOptions {
                            text_document_registration_options: {
                                TextDocumentRegistrationOptions {
                                    document_selector: Some(vec![DocumentFilter {
                                        language: Some("gen".to_string()),
                                        scheme: Some("file".to_string()),
                                        pattern: Some("*.gen".to_string()),
                                    }]),
                                }
                            },
                            semantic_tokens_options: SemanticTokensOptions {
                                legend: SemanticTokensLegend {
                                    token_types: LEGEND_TYPE.into(),
                                    token_modifiers: vec![],
                                },
                                range: Some(true),
                                full: Some(SemanticTokensFullOptions::Bool(true)),
                                work_done_progress_options: WorkDoneProgressOptions::default(),
                            },
                            static_registration_options: StaticRegistrationOptions::default(),
                        },
                    ),
                ),

                definition_provider: Some(OneOf::Left(true)),
                references_provider: Some(OneOf::Left(true)),
                rename_provider: Some(OneOf::Left(true)),
                document_formatting_provider: Some(OneOf::Left(true)),
                document_range_formatting_provider: Some(OneOf::Left(true)),
                ..ServerCapabilities::default()
            },
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "Server initialized")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        self.client
            .log_message(MessageType::INFO, "Server shutdown")
            .await;
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.client
            .log_message(
                MessageType::INFO,
                format!(
                    "{} file opened with version {}",
                    params.text_document.uri, params.text_document.version
                ),
            )
            .await;

        self.run_diagnostics(TextDocumentItem {
            uri: params.text_document.uri,
            version: params.text_document.version,
        })
        .await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        self.client
            .log_message(
                MessageType::INFO,
                format!(
                    "{} file changes with version {}\nChanges:\n{}",
                    params.text_document.uri,
                    params.text_document.version,
                    params
                        .content_changes
                        .iter()
                        .map(|c| format!(
                            "From {:?} to {:?} -> {}",
                            c.range.unwrap().start,
                            c.range.unwrap().end,
                            c.text
                        ))
                        .collect::<Vec<_>>()
                        .join("\n")
                ),
            )
            .await;
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        self.client
            .log_message(
                MessageType::INFO,
                format!("{} file saved", params.text_document.uri),
            )
            .await;

        if let Some(text) = params.text {
            self.client
                .log_message(MessageType::INFO, format!("With new text:\n{}", text))
                .await;
            self.run_diagnostics(TextDocumentItem {
                uri: params.text_document.uri,
                version: 0,
            })
            .await;
        };
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        self.client
            .log_message(
                MessageType::INFO,
                format!("{} file closed", params.text_document.uri),
            )
            .await;
    }

    /// Gets a file and location of an element
    /// Returns the file and localtion where it was defined
    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let definition = async {
            // Get the path of the file that was modified
            let uri = params.text_document_position_params.text_document.uri;

            // Get origin location that triggered the event
            let range = Range::new(
                params.text_document_position_params.position,
                params.text_document_position_params.position,
            );

            self.client
                .log_message(
                    MessageType::INFO,
                    format!(
                        "{} file trigers GoToDefinition from: {:?}",
                        uri, params.text_document_position_params.position
                    ),
                )
                .await;

            // Find out where it's defind and retour its location (sending same as exemple)
            Some(GotoDefinitionResponse::Scalar(Location::new(uri, range)))
        }
        .await;
        Ok(definition)
    }

    /// Gets an element to look for its references, it has a flag saying if the declarion is included
    /// Returns a list of positions where this elements is referenced
    async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        let uri = params.text_document_position.text_document.uri;
        let range = Range::new(
            params.text_document_position.position,
            params.text_document_position.position,
        );

        self.client
            .log_message(
                MessageType::INFO,
                format!(
                    "{} file trigers GoToDefinition from: {:?}",
                    uri, params.text_document_position.position
                ),
            )
            .await;

        Ok(Some(vec![Location::new(uri, range)]))
    }

    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> Result<Option<SemanticTokensResult>> {
        let uri = params.text_document.uri.to_string();
        self.client
            .log_message(MessageType::LOG, format!("{} Semantic tokens full", uri))
            .await;
        Ok(None)
    }

    async fn semantic_tokens_range(
        &self,
        _params: SemanticTokensRangeParams,
    ) -> Result<Option<SemanticTokensRangeResult>> {
        Ok(None)
    }

    async fn inlay_hint(
        &self,
        _params: tower_lsp::lsp_types::InlayHintParams,
    ) -> Result<Option<Vec<InlayHint>>> {
        self.client
            .log_message(MessageType::INFO, "inlay hint")
            .await;
        Ok(None)
    }

    async fn completion(&self, _params: CompletionParams) -> Result<Option<CompletionResponse>> {
        Ok(None)
    }

    async fn rename(&self, _params: RenameParams) -> Result<Option<WorkspaceEdit>> {
        Ok(None)
    }

    async fn formatting(&self, params: DocumentFormattingParams) -> Result<Option<Vec<TextEdit>>> {
        self.client
            .log_message(
                MessageType::INFO,
                format!(
                    "{} requested formatting with: {:?}",
                    params.text_document.uri, params.options
                ),
            )
            .await;
        Ok(None)
    }

    async fn range_formatting(
        &self,
        params: DocumentRangeFormattingParams,
    ) -> Result<Option<Vec<TextEdit>>> {
        self.client
            .log_message(
                MessageType::INFO,
                format!(
                    "{} requested range formatting with: {:?}",
                    params.text_document.uri, params.options
                ),
            )
            .await;
        Ok(None)
    }

    async fn did_change_configuration(&self, _: DidChangeConfigurationParams) {
        self.client
            .log_message(MessageType::INFO, "configuration changed")
            .await;
    }

    async fn did_change_workspace_folders(&self, _: DidChangeWorkspaceFoldersParams) {
        self.client
            .log_message(MessageType::INFO, "workspace folders changed")
            .await;
    }

    async fn did_change_watched_files(&self, params: DidChangeWatchedFilesParams) {
        self.client
            .log_message(
                MessageType::INFO,
                format!(
                    "Watched files have changed:\n{}",
                    params
                        .changes
                        .iter()
                        .map(|w| format!("{:?} - {}", w.typ, w.uri))
                        .collect::<Vec<_>>()
                        .join("\n")
                ),
            )
            .await;
    }

    async fn execute_command(&self, params: ExecuteCommandParams) -> Result<Option<Value>> {
        self.client
            .log_message(
                MessageType::INFO,
                format!("{} command executed", params.command),
            )
            .await;

        Ok(None)
    }
}
#[derive(Debug, Deserialize, Serialize)]
struct InlayHintParams {
    path: String,
}

enum CustomNotification {}
impl Notification for CustomNotification {
    type Params = InlayHintParams;
    const METHOD: &'static str = "custom/notification";
}
struct TextDocumentItem {
    uri: Url,
    version: i32,
}
impl Backend {
    async fn run_diagnostics(&self, params: TextDocumentItem) {
        let pos = Position::new(0, 0);
        let diagnostics = vec![Diagnostic::new_simple(
            Range::new(pos, pos),
            "error".to_string(),
        )];

        self.client
            .publish_diagnostics(params.uri.clone(), diagnostics, Some(params.version))
            .await;
    }
}

#[tokio::main]
async fn main() {
    env_logger::init();

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::build(|client| Backend { client }).finish();

    Server::new(stdin, stdout, socket).serve(service).await;
}
