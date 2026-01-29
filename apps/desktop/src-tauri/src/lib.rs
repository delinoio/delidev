use std::sync::Arc;

use sentry_tracing::EventFilter;
use tauri::Manager;

pub mod commands;
pub mod config;
pub mod database;
pub mod entities;
pub mod services;

use services::AppState;

const SENTRY_DSN: &str =
    "https://9930ad2c1205512beb8738957c0a0271@o4510703761227776.ingest.us.sentry.io/4510703764635648";

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize Sentry for error tracking
    let traces_sample_rate = std::env::var("SENTRY_TRACES_SAMPLE_RATE")
        .ok()
        .and_then(|s| s.parse::<f32>().ok())
        .unwrap_or(0.1);

    {
        let guard = sentry::init((
            SENTRY_DSN,
            sentry::ClientOptions {
                release: sentry::release_name!(),
                traces_sample_rate,
                ..Default::default()
            },
        ));
        std::mem::forget(guard);
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_process::init())
        .invoke_handler(tauri::generate_handler![
            // System commands
            commands::check_docker,
            commands::get_docker_version,
            commands::get_app_info,
            commands::open_in_editor,
            // Config commands
            commands::get_global_config,
            commands::update_global_config,
            commands::get_repository_config,
            commands::update_repository_config,
            commands::get_credentials_status,
            commands::set_github_token,
            commands::set_gitlab_token,
            commands::set_bitbucket_credentials,
            commands::validate_vcs_credentials,
            // Repository commands
            commands::list_repositories,
            commands::get_repository,
            commands::add_repository,
            commands::remove_repository,
            commands::validate_repository_path,
            commands::list_repository_files,
            // Task commands
            commands::list_unit_tasks,
            commands::get_unit_task,
            commands::get_agent_task,
            commands::create_unit_task,
            commands::update_unit_task_status,
            commands::rename_unit_task_branch,
            commands::request_unit_task_changes,
            commands::delete_unit_task,
            commands::list_composite_tasks,
            commands::get_composite_task,
            commands::create_composite_task,
            commands::update_composite_task_status,
            commands::delete_composite_task,
            commands::get_tasks_by_status,
            // Execution commands
            commands::start_task_execution,
            commands::stop_task_execution,
            commands::get_execution_logs,
            commands::get_all_execution_logs,
            commands::get_stream_messages,
            commands::get_historical_execution_logs,
            commands::cleanup_task,
            commands::is_docker_available,
            commands::is_task_executing,
            commands::get_task_diff,
            commands::create_pr_for_task,
            commands::commit_to_repository,
            // Token usage commands
            commands::get_session_usage,
            commands::get_unit_task_usage,
            commands::get_composite_task_usage,
            // Composite task planning commands
            commands::start_composite_task_planning,
            commands::get_composite_task_plan,
            commands::approve_composite_task_plan,
            commands::reject_composite_task_plan,
            commands::update_composite_task_plan,
            commands::execute_composite_task_nodes,
            // Custom command commands
            commands::list_custom_commands,
            commands::get_custom_command,
            commands::list_custom_commands_by_agent,
            commands::list_custom_commands_by_framework,
            commands::render_custom_command,
            // Workspace commands
            commands::list_workspaces,
            commands::get_workspace,
            commands::create_workspace,
            commands::update_workspace,
            commands::delete_workspace,
            commands::add_repository_to_workspace,
            commands::remove_repository_from_workspace,
            commands::list_workspace_repositories,
            commands::get_default_workspace,
            // Repository group commands
            commands::list_repository_groups,
            commands::get_repository_group,
            commands::create_repository_group,
            commands::update_repository_group,
            commands::delete_repository_group,
            commands::add_repository_to_group,
            commands::remove_repository_from_group,
            commands::get_or_create_single_repo_group,
            commands::list_repository_group_files,
        ])
        .setup(|app| {
            // Initialize tracing with Sentry integration
            use tracing_subscriber::prelude::*;

            let sentry_layer = sentry_tracing::layer().event_filter(|md| match *md.level() {
                tracing::Level::ERROR | tracing::Level::WARN => EventFilter::Event,
                _ => EventFilter::Ignore,
            });

            tracing_subscriber::registry()
                .with(
                    tracing_subscriber::fmt::layer().with_filter(
                        tracing_subscriber::EnvFilter::from_default_env()
                            .add_directive(tracing::Level::INFO.into()),
                    ),
                )
                .with(sentry_layer)
                .init();

            // Initialize application state
            let app_handle = app.handle().clone();
            let state = tauri::async_runtime::block_on(async {
                let state = AppState::new(Some(app_handle.clone())).await?;
                Ok::<_, anyhow::Error>(state)
            });

            match state {
                Ok(state) => {
                    let state = Arc::new(state);
                    // Start the pending task handler for concurrency queue
                    tauri::async_runtime::spawn({
                        let state = Arc::clone(&state);
                        async move {
                            state.start_pending_task_handler().await;
                        }
                    });
                    app_handle.manage(state);
                    tracing::info!("Application state initialized successfully");
                }
                Err(e) => {
                    tracing::error!("Failed to initialize application state: {}", e);
                    return Err(format!("Failed to initialize application: {}", e).into());
                }
            }

            // Register global shortcut plugin
            #[cfg(desktop)]
            {
                use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

                let shortcut = Shortcut::new(
                    Some(tauri_plugin_global_shortcut::Modifiers::ALT),
                    tauri_plugin_global_shortcut::Code::KeyZ,
                );
                app.handle().plugin(
                    tauri_plugin_global_shortcut::Builder::new()
                        .with_handler(move |_app, _shortcut, event| {
                            if event.state() == tauri_plugin_global_shortcut::ShortcutState::Pressed
                            {
                                // TODO: Open chat window
                                tracing::info!("Global shortcut pressed: Alt+Z");
                            }
                        })
                        .build(),
                )?;

                // Register the shortcut
                app.global_shortcut().register(shortcut)?;
            }

            #[cfg(debug_assertions)]
            {
                let window = app.get_webview_window("main").unwrap();
                window.open_devtools();
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
