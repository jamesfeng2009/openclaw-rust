use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpBuiltinTool {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
    pub category: McpToolCategory,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum McpToolCategory {
    FileSystem,
    Network,
    Process,
    Browser,
    Database,
    Git,
    Container,
    Cloud,
    Utils,
}

pub struct McpBuiltinTools;

impl McpBuiltinTools {
    pub fn all() -> Vec<McpBuiltinTool> {
        let mut tools = Vec::new();
        tools.extend(Self::filesystem_tools());
        tools.extend(Self::network_tools());
        tools.extend(Self::process_tools());
        tools.extend(Self::browser_tools());
        tools.extend(Self::database_tools());
        tools.extend(Self::git_tools());
        tools.extend(Self::container_tools());
        tools.extend(Self::cloud_tools());
        tools.extend(Self::utils_tools());
        tools
    }

    pub fn filesystem_tools() -> Vec<McpBuiltinTool> {
        vec![
            McpBuiltinTool {
                name: "read_file".to_string(),
                description: "Read contents of a file from the filesystem".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Absolute path to the file"
                        },
                        "offset": {
                            "type": "number",
                            "description": "Byte offset to start reading from"
                        },
                        "length": {
                            "type": "number",
                            "description": "Maximum number of bytes to read"
                        }
                    },
                    "required": ["path"]
                }),
                category: McpToolCategory::FileSystem,
            },
            McpBuiltinTool {
                name: "write_file".to_string(),
                description: "Write content to a file".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Absolute path to the file"
                        },
                        "content": {
                            "type": "string",
                            "description": "Content to write to the file"
                        },
                        "append": {
                            "type": "boolean",
                            "description": "Append to file instead of overwriting",
                            "default": false
                        }
                    },
                    "required": ["path", "content"]
                }),
                category: McpToolCategory::FileSystem,
            },
            McpBuiltinTool {
                name: "list_directory".to_string(),
                description: "List files and directories in a path".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Directory path to list"
                        },
                        "recursive": {
                            "type": "boolean",
                            "description": "List subdirectories recursively",
                            "default": false
                        }
                    },
                    "required": ["path"]
                }),
                category: McpToolCategory::FileSystem,
            },
            McpBuiltinTool {
                name: "create_directory".to_string(),
                description: "Create a new directory".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Directory path to create"
                        },
                        "recursive": {
                            "type": "boolean",
                            "description": "Create parent directories if they don't exist",
                            "default": true
                        }
                    },
                    "required": ["path"]
                }),
                category: McpToolCategory::FileSystem,
            },
            McpBuiltinTool {
                name: "delete_path".to_string(),
                description: "Delete a file or directory".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Path to delete"
                        },
                        "recursive": {
                            "type": "boolean",
                            "description": "Delete directories recursively",
                            "default": false
                        }
                    },
                    "required": ["path"]
                }),
                category: McpToolCategory::FileSystem,
            },
            McpBuiltinTool {
                name: "move_path".to_string(),
                description: "Move or rename a file or directory".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "source": {
                            "type": "string",
                            "description": "Source path"
                        },
                        "destination": {
                            "type": "string",
                            "description": "Destination path"
                        }
                    },
                    "required": ["source", "destination"]
                }),
                category: McpToolCategory::FileSystem,
            },
            McpBuiltinTool {
                name: "copy_path".to_string(),
                description: "Copy a file or directory".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "source": {
                            "type": "string",
                            "description": "Source path"
                        },
                        "destination": {
                            "type": "string",
                            "description": "Destination path"
                        }
                    },
                    "required": ["source", "destination"]
                }),
                category: McpToolCategory::FileSystem,
            },
            McpBuiltinTool {
                name: "get_path_info".to_string(),
                description: "Get information about a file or directory".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Path to get information about"
                        }
                    },
                    "required": ["path"]
                }),
                category: McpToolCategory::FileSystem,
            },
            McpBuiltinTool {
                name: "search_files".to_string(),
                description: "Search for files matching a pattern".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Directory to search in"
                        },
                        "pattern": {
                            "type": "string",
                            "description": "Search pattern (glob or regex)"
                        },
                        "file_type": {
                            "type": "string",
                            "description": "Filter by file type: file, directory, or any",
                            "enum": ["file", "directory", "any"],
                            "default": "any"
                        }
                    },
                    "required": ["path", "pattern"]
                }),
                category: McpToolCategory::FileSystem,
            },
        ]
    }

    pub fn network_tools() -> Vec<McpBuiltinTool> {
        vec![
            McpBuiltinTool {
                name: "http_request".to_string(),
                description: "Make an HTTP request".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "method": {
                            "type": "string",
                            "description": "HTTP method",
                            "enum": ["GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS"]
                        },
                        "url": {
                            "type": "string",
                            "description": "Request URL"
                        },
                        "headers": {
                            "type": "object",
                            "description": "Request headers"
                        },
                        "body": {
                            "type": "string",
                            "description": "Request body"
                        },
                        "timeout": {
                            "type": "number",
                            "description": "Request timeout in seconds",
                            "default": 30
                        }
                    },
                    "required": ["method", "url"]
                }),
                category: McpToolCategory::Network,
            },
            McpBuiltinTool {
                name: "fetch_url".to_string(),
                description: "Fetch content from a URL".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "url": {
                            "type": "string",
                            "description": "URL to fetch"
                        },
                        "accept": {
                            "type": "string",
                            "description": "Accept header",
                            "default": "*/*"
                        }
                    },
                    "required": ["url"]
                }),
                category: McpToolCategory::Network,
            },
        ]
    }

    pub fn process_tools() -> Vec<McpBuiltinTool> {
        vec![
            McpBuiltinTool {
                name: "execute_command".to_string(),
                description: "Execute a shell command".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "command": {
                            "type": "string",
                            "description": "Command to execute"
                        },
                        "args": {
                            "type": "array",
                            "items": {"type": "string"},
                            "description": "Command arguments"
                        },
                        "cwd": {
                            "type": "string",
                            "description": "Working directory"
                        },
                        "env": {
                            "type": "object",
                            "description": "Environment variables"
                        },
                        "timeout": {
                            "type": "number",
                            "description": "Timeout in seconds",
                            "default": 60
                        }
                    },
                    "required": ["command"]
                }),
                category: McpToolCategory::Process,
            },
            McpBuiltinTool {
                name: "list_processes".to_string(),
                description: "List running processes".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "filter": {
                            "type": "string",
                            "description": "Filter processes by name"
                        }
                    }
                }),
                category: McpToolCategory::Process,
            },
            McpBuiltinTool {
                name: "kill_process".to_string(),
                description: "Terminate a process".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "pid": {
                            "type": "number",
                            "description": "Process ID to kill"
                        },
                        "force": {
                            "type": "boolean",
                            "description": "Force kill",
                            "default": false
                        }
                    },
                    "required": ["pid"]
                }),
                category: McpToolCategory::Process,
            },
        ]
    }

    pub fn browser_tools() -> Vec<McpBuiltinTool> {
        vec![
            McpBuiltinTool {
                name: "browser_navigate".to_string(),
                description: "Navigate to a URL in browser".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "url": {
                            "type": "string",
                            "description": "URL to navigate to"
                        },
                        "new_tab": {
                            "type": "boolean",
                            "description": "Open in new tab",
                            "default": false
                        }
                    },
                    "required": ["url"]
                }),
                category: McpToolCategory::Browser,
            },
            McpBuiltinTool {
                name: "browser_screenshot".to_string(),
                description: "Take a screenshot of the current page".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "full_page": {
                            "type": "boolean",
                            "description": "Capture full page",
                            "default": false
                        }
                    }
                }),
                category: McpToolCategory::Browser,
            },
            McpBuiltinTool {
                name: "browser_click".to_string(),
                description: "Click an element on the page".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "selector": {
                            "type": "string",
                            "description": "CSS selector or XPath"
                        },
                        "x": {
                            "type": "number",
                            "description": "X coordinate"
                        },
                        "y": {
                            "type": "number",
                            "description": "Y coordinate"
                        }
                    }
                }),
                category: McpToolCategory::Browser,
            },
            McpBuiltinTool {
                name: "browser_type".to_string(),
                description: "Type text into an element".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "selector": {
                            "type": "string",
                            "description": "CSS selector or XPath"
                        },
                        "text": {
                            "type": "string",
                            "description": "Text to type"
                        },
                        "clear": {
                            "type": "boolean",
                            "description": "Clear before typing",
                            "default": false
                        }
                    },
                    "required": ["text"]
                }),
                category: McpToolCategory::Browser,
            },
            McpBuiltinTool {
                name: "browser_evaluate".to_string(),
                description: "Execute JavaScript in browser context".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "script": {
                            "type": "string",
                            "description": "JavaScript code to execute"
                        }
                    },
                    "required": ["script"]
                }),
                category: McpToolCategory::Browser,
            },
            McpBuiltinTool {
                name: "browser_get_html".to_string(),
                description: "Get HTML content of the page".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "selector": {
                            "type": "string",
                            "description": "Optional CSS selector to get specific element"
                        }
                    }
                }),
                category: McpToolCategory::Browser,
            },
        ]
    }

    pub fn database_tools() -> Vec<McpBuiltinTool> {
        vec![
            McpBuiltinTool {
                name: "db_query".to_string(),
                description: "Execute a database query".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "connection_string": {
                            "type": "string",
                            "description": "Database connection string"
                        },
                        "query": {
                            "type": "string",
                            "description": "SQL query to execute"
                        },
                        "params": {
                            "type": "array",
                            "description": "Query parameters"
                        }
                    },
                    "required": ["query"]
                }),
                category: McpToolCategory::Database,
            },
            McpBuiltinTool {
                name: "db_execute".to_string(),
                description: "Execute a database statement (INSERT, UPDATE, DELETE)".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "connection_string": {
                            "type": "string",
                            "description": "Database connection string"
                        },
                        "statement": {
                            "type": "string",
                            "description": "SQL statement to execute"
                        },
                        "params": {
                            "type": "array",
                            "description": "Statement parameters"
                        }
                    },
                    "required": ["statement"]
                }),
                category: McpToolCategory::Database,
            },
            McpBuiltinTool {
                name: "db_list_tables".to_string(),
                description: "List tables in a database".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "connection_string": {
                            "type": "string",
                            "description": "Database connection string"
                        }
                    },
                    "required": ["connection_string"]
                }),
                category: McpToolCategory::Database,
            },
        ]
    }

    pub fn git_tools() -> Vec<McpBuiltinTool> {
        vec![
            McpBuiltinTool {
                name: "git_status".to_string(),
                description: "Get git repository status".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "repo_path": {
                            "type": "string",
                            "description": "Path to git repository"
                        }
                    }
                }),
                category: McpToolCategory::Git,
            },
            McpBuiltinTool {
                name: "git_log".to_string(),
                description: "Get git commit log".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "repo_path": {
                            "type": "string",
                            "description": "Path to git repository"
                        },
                        "max_count": {
                            "type": "number",
                            "description": "Maximum number of commits",
                            "default": 10
                        }
                    }
                }),
                category: McpToolCategory::Git,
            },
            McpBuiltinTool {
                name: "git_diff".to_string(),
                description: "Get git diff".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "repo_path": {
                            "type": "string",
                            "description": "Path to git repository"
                        },
                        "commit": {
                            "type": "string",
                            "description": "Commit SHA or reference"
                        }
                    }
                }),
                category: McpToolCategory::Git,
            },
            McpBuiltinTool {
                name: "git_branch".to_string(),
                description: "List git branches".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "repo_path": {
                            "type": "string",
                            "description": "Path to git repository"
                        }
                    }
                }),
                category: McpToolCategory::Git,
            },
        ]
    }

    pub fn container_tools() -> Vec<McpBuiltinTool> {
        vec![
            McpBuiltinTool {
                name: "container_list".to_string(),
                description: "List containers".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "all": {
                            "type": "boolean",
                            "description": "Show all containers",
                            "default": false
                        }
                    }
                }),
                category: McpToolCategory::Container,
            },
            McpBuiltinTool {
                name: "container_logs".to_string(),
                description: "Get container logs".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "container_id": {
                            "type": "string",
                            "description": "Container ID or name"
                        },
                        "tail": {
                            "type": "number",
                            "description": "Number of lines to show",
                            "default": 100
                        }
                    },
                    "required": ["container_id"]
                }),
                category: McpToolCategory::Container,
            },
            McpBuiltinTool {
                name: "container_exec".to_string(),
                description: "Execute command in container".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "container_id": {
                            "type": "string",
                            "description": "Container ID or name"
                        },
                        "command": {
                            "type": "string",
                            "description": "Command to execute"
                        }
                    },
                    "required": ["container_id", "command"]
                }),
                category: McpToolCategory::Container,
            },
        ]
    }

    pub fn cloud_tools() -> Vec<McpBuiltinTool> {
        vec![
            McpBuiltinTool {
                name: "cloud_list_instances".to_string(),
                description: "List cloud compute instances".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "provider": {
                            "type": "string",
                            "description": "Cloud provider: aws, gcp, azure",
                            "enum": ["aws", "gcp", "azure"]
                        },
                        "region": {
                            "type": "string",
                            "description": "Region filter"
                        }
                    },
                    "required": ["provider"]
                }),
                category: McpToolCategory::Cloud,
            },
            McpBuiltinTool {
                name: "cloud_deploy".to_string(),
                description: "Deploy to cloud".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "provider": {
                            "type": "string",
                            "description": "Cloud provider",
                            "enum": ["aws", "gcp", "azure"]
                        },
                        "service": {
                            "type": "string",
                            "description": "Service to deploy to"
                        },
                        "image": {
                            "type": "string",
                            "description": "Container image"
                        }
                    },
                    "required": ["provider", "service"]
                }),
                category: McpToolCategory::Cloud,
            },
        ]
    }

    pub fn utils_tools() -> Vec<McpBuiltinTool> {
        vec![
            McpBuiltinTool {
                name: "get_system_info".to_string(),
                description: "Get system information".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {}
                }),
                category: McpToolCategory::Utils,
            },
            McpBuiltinTool {
                name: "get_current_time".to_string(),
                description: "Get current time".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "timezone": {
                            "type": "string",
                            "description": "Timezone (e.g., UTC, America/New_York)",
                            "default": "UTC"
                        }
                    }
                }),
                category: McpToolCategory::Utils,
            },
            McpBuiltinTool {
                name: "hash_content".to_string(),
                description: "Calculate hash of content".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "content": {
                            "type": "string",
                            "description": "Content to hash"
                        },
                        "algorithm": {
                            "type": "string",
                            "description": "Hash algorithm",
                            "enum": ["md5", "sha1", "sha256", "sha512"],
                            "default": "sha256"
                        }
                    },
                    "required": ["content"]
                }),
                category: McpToolCategory::Utils,
            },
            McpBuiltinTool {
                name: "encode_base64".to_string(),
                description: "Encode string to base64".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "content": {
                            "type": "string",
                            "description": "Content to encode"
                        }
                    },
                    "required": ["content"]
                }),
                category: McpToolCategory::Utils,
            },
            McpBuiltinTool {
                name: "decode_base64".to_string(),
                description: "Decode base64 string".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "content": {
                            "type": "string",
                            "description": "Base64 content to decode"
                        }
                    },
                    "required": ["content"]
                }),
                category: McpToolCategory::Utils,
            },
            McpBuiltinTool {
                name: "encode_json".to_string(),
                description: "Encode value as JSON string".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "value": {
                            "type": "object",
                            "description": "Value to encode"
                        }
                    },
                    "required": ["value"]
                }),
                category: McpToolCategory::Utils,
            },
            McpBuiltinTool {
                name: "decode_json".to_string(),
                description: "Decode JSON string to value".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "content": {
                            "type": "string",
                            "description": "JSON string to decode"
                        }
                    },
                    "required": ["content"]
                }),
                category: McpToolCategory::Utils,
            },
        ]
    }

    pub fn by_category(category: McpToolCategory) -> Vec<McpBuiltinTool> {
        match category {
            McpToolCategory::FileSystem => Self::filesystem_tools(),
            McpToolCategory::Network => Self::network_tools(),
            McpToolCategory::Process => Self::process_tools(),
            McpToolCategory::Browser => Self::browser_tools(),
            McpToolCategory::Database => Self::database_tools(),
            McpToolCategory::Git => Self::git_tools(),
            McpToolCategory::Container => Self::container_tools(),
            McpToolCategory::Cloud => Self::cloud_tools(),
            McpToolCategory::Utils => Self::utils_tools(),
        }
    }
}
