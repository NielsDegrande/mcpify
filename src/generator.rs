use serde_json::Value;

pub struct CodeGenerator {
    openapi: Value,
}

impl CodeGenerator {
    pub fn new(openapi: Value) -> Self {
        Self { openapi }
    }

    pub fn generate(&self) -> String {
        let mut code = String::new();

        self.add_imports(&mut code);
        self.generate_tools(&mut code);
        self.add_server_connection(&mut code);

        code
    }

    /// Appends the TypeScript import statements and initialization code to the provided string.
    ///
    /// This function writes the necessary import statements, environment configuration, and
    /// the backend call helper function to the `code` string. It also initializes the MCP server
    /// object. This setup is required for the generated TypeScript server code to function correctly.
    ///
    /// # Arguments
    ///
    /// * `code` - A mutable reference to the string where the generated TypeScript code will be appended.
    fn add_imports(&self, code: &mut String) {
        code.push_str(
            r#"/**
 * Generated MCP server from OpenAPI spec.
 */

import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import dotenv from "dotenv";
import { z } from "zod";

dotenv.config();

/**
 * Calls the backend REST API.
 */
async function callBackend<T>(path: string, options?: RequestInit): Promise<T> {
  const baseUrl = process.env.BACKEND_URL;
  const url = `${baseUrl}${path}`;
  const res = await fetch(url, options);
  if (!res.ok) {
    throw new Error(`Backend error: ${res.status} ${res.statusText}`);
  }
  return res.json();
}

const server = new McpServer({
  name: "Generated-MCP",
  version: "1.0.0",
});
"#,
        );
    }

    /// Generates TypeScript server tool functions for all operations defined in the OpenAPI specification.
    ///
    /// This function iterates over all paths and HTTP methods in the OpenAPI document and generates
    /// corresponding TypeScript server tool code for each operation. The generated code is appended
    /// to the provided `code` string.
    ///
    /// # Arguments
    ///
    /// * `code` - A mutable reference to the string where the generated TypeScript code will be appended.
    fn generate_tools(&self, code: &mut String) {
        if let Some(paths) = self.openapi.get("paths") {
            if let Some(paths_obj) = paths.as_object() {
                for (path, path_item) in paths_obj {
                    if let Some(path_item_obj) = path_item.as_object() {
                        for (method, operation) in path_item_obj {
                            if let Some(operation_obj) = operation.as_object() {
                                let operation_value = Value::Object(operation_obj.clone());
                                self.generate_tool(code, path, method, &operation_value);
                            }
                        }
                    }
                }
            }
        }
    }

    /// Generates a TypeScript server tool function for a given OpenAPI operation.
    ///
    /// This function appends the generated TypeScript code for a server tool to the provided `code` string.
    /// It uses the specified HTTP path, method, and operation details from the OpenAPI specification.
    ///
    /// # Arguments
    ///
    /// * `code` - A mutable reference to the string where the generated TypeScript code will be appended.
    /// * `path` - The HTTP path for the operation (e.g., "/agents/{id}").
    /// * `method` - The HTTP method for the operation (e.g., "get", "post").
    /// * `operation` - The OpenAPI operation object as a serde_json `Value`.
    fn generate_tool(&self, code: &mut String, path: &str, method: &str, operation: &Value) {
        let operation_id = operation
            .get("operationId")
            .and_then(|id| id.as_str())
            .map(String::from)
            .unwrap_or_else(|| format!("{}_{}", method, path.replace('/', "_")));

        let params = self.collect_parameters(operation);
        let has_query_params = params.iter().any(|p| p.contains("query"));

        // Generate tool.
        code.push_str(&format!(
            "\nserver.tool(\n  \"{}\",\n  {{\n    {}\n  }},\n  async (params) => {{\n",
            operation_id,
            params.join(",\n    ")
        ));

        // Add query parameters only if they exist.
        if has_query_params {
            code.push_str("    const search = new URLSearchParams();\n");
            code.push_str("    Object.entries(params).forEach(([key, value]) => {\n");
            code.push_str("      if (value) search.set(key, String(value));\n");
            code.push_str("    });\n\n");
        }

        // Add API call.
        let method_upper = method.to_uppercase();
        let request_options = match method_upper.as_str() {
            "GET" => "{\n        method: \"GET\"\n      }".to_string(),
            "POST" | "PUT" | "PATCH" => {
                format!(
                    "{{\n        method: \"{}\",\n        headers: {{ \"Content-Type\": \"application/json\" }},\n        body: JSON.stringify(params)\n      }}",
                    method_upper
                )
            }
            "DELETE" => {
                if !params.is_empty() {
                    "{\n        method: \"DELETE\",\n        headers: { \"Content-Type\": \"application/json\" },\n        body: JSON.stringify(params)\n      }".to_string()
                } else {
                    "{\n        method: \"DELETE\"\n      }".to_string()
                }
            }
            _ => {
                format!("{{\n        method: \"{}\"\n      }}", method_upper)
            }
        };

        code.push_str(&format!(
            "    const result = await callBackend<any>(\n      \"{}{}\",\n      {}\n    );\n\n",
            path,
            if has_query_params {
                "?${search.toString()}"
            } else {
                ""
            },
            request_options
        ));

        // Add response.
        code.push_str("    return {\n");
        code.push_str("      content: [\n");
        code.push_str("        {\n");
        code.push_str("          type: \"text\",\n");
        code.push_str("          text: JSON.stringify(result, null, 2),\n");
        code.push_str("        },\n");
        code.push_str("      ],\n");
        code.push_str("    };\n");
        code.push_str("  }\n);\n");
    }

    /// Collects the parameters for a given OpenAPI operation and returns them as a vector of strings
    /// formatted for use with the Zod schema in TypeScript code generation.
    ///
    /// This function inspects the provided OpenAPI operation object and extracts both query parameters
    /// and request body properties (if present). Query parameters are added as optional strings,
    /// while request body properties are added as required strings.
    ///
    /// # Arguments
    ///
    /// * `operation` - A reference to a serde_json::Value representing the OpenAPI operation object.
    ///
    /// # Returns
    ///
    /// A vector of strings, each representing a parameter definition suitable for use in a Zod schema.
    fn collect_parameters(&self, operation: &Value) -> Vec<String> {
        let mut params = Vec::new();

        // Collect query parameters.
        if let Some(parameters) = operation.get("parameters") {
            if let Some(params_array) = parameters.as_array() {
                for param in params_array {
                    if let Some(param_obj) = param.as_object() {
                        if let (Some(name), Some(in_)) = (
                            param_obj.get("name").and_then(|n| n.as_str()),
                            param_obj.get("in").and_then(|i| i.as_str()),
                        ) {
                            if in_ == "query" {
                                params.push(format!("{}: z.string().optional()", name));
                            }
                        }
                    }
                }
            }
        }

        // Collect request body parameters.
        if let Some(request_body) = operation.get("requestBody") {
            if let Some(content) = request_body.get("content") {
                if let Some(json) = content.get("application/json") {
                    if let Some(schema) = json.get("schema") {
                        if let Some(ref_path) = schema.get("$ref").and_then(|r| r.as_str()) {
                            // Handle schema reference.
                            if let Some(components) = self.openapi.get("components") {
                                if let Some(schemas) = components.get("schemas") {
                                    if let Some(referenced_schema) = schemas
                                        .get(ref_path.trim_start_matches("#/components/schemas/"))
                                    {
                                        self.process_schema_properties(
                                            referenced_schema,
                                            &mut params,
                                        );
                                    }
                                }
                            }
                        } else {
                            self.process_schema_properties(schema, &mut params);
                        }
                    }
                }
            }
        }

        params
    }

    /// Processes the properties of a schema and adds them to the parameters list.
    ///
    /// # Arguments
    ///
    /// * `schema` - The schema object containing properties to process.
    /// * `params` - A mutable reference to the vector where parameter definitions will be added.
    fn process_schema_properties(&self, schema: &Value, params: &mut Vec<String>) {
        if let Some(properties) = schema.get("properties") {
            if let Some(props_obj) = properties.as_object() {
                let required = schema
                    .get("required")
                    .and_then(|r| r.as_array())
                    .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>())
                    .unwrap_or_default();

                for (prop_name, prop_schema) in props_obj {
                    let type_def = self.get_type_definition(prop_schema);
                    let is_required = required.contains(&prop_name.as_str());
                    let param_def = if is_required {
                        format!("{}: {}", prop_name, type_def)
                    } else {
                        format!("{}: {}.optional()", prop_name, type_def)
                    };
                    params.push(param_def);
                }
            }
        }
    }

    /// Determines the Zod type definition for a given schema.
    ///
    /// # Arguments
    ///
    /// * `schema` - The schema object to determine the type for.
    ///
    /// # Returns
    ///
    /// A string representing the Zod type definition.
    fn get_type_definition(&self, schema: &Value) -> String {
        match schema.get("type").and_then(|t| t.as_str()) {
            Some("string") => "z.string()".to_string(),
            Some("number") => "z.number()".to_string(),
            Some("integer") => "z.number().int()".to_string(),
            Some("boolean") => "z.boolean()".to_string(),
            Some("array") => {
                if let Some(items) = schema.get("items") {
                    if let Some(item_type) = items.get("type").and_then(|t| t.as_str()) {
                        match item_type {
                            "string" => "z.array(z.string())".to_string(),
                            "number" => "z.array(z.number())".to_string(),
                            "integer" => "z.array(z.number().int())".to_string(),
                            "boolean" => "z.array(z.boolean())".to_string(),
                            _ => "z.array(z.any())".to_string(),
                        }
                    } else {
                        "z.array(z.any())".to_string()
                    }
                } else {
                    "z.array(z.any())".to_string()
                }
            }
            _ => "z.any()".to_string(),
        }
    }

    /// Appends the TypeScript code required to establish a server connection using
    /// the StdioServerTransport and connect it to the generated server.
    ///
    /// # Arguments
    ///
    /// * `code` - A mutable reference to the String where the generated TypeScript code is appended.
    fn add_server_connection(&self, code: &mut String) {
        code.push_str(
            "\nconst transport = new StdioServerTransport();\nawait server.connect(transport);\n",
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn create_test_openapi() -> Value {
        json!({
            "paths": {
                "/test": {
                    "get": {
                        "operationId": "testOperation",
                        "parameters": [
                            {
                                "name": "queryParam",
                                "in": "query",
                                "schema": {
                                    "type": "string"
                                }
                            }
                        ],
                        "requestBody": {
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "type": "object",
                                        "properties": {
                                            "requiredField": {
                                                "type": "string"
                                            },
                                            "optionalField": {
                                                "type": "number"
                                            }
                                        },
                                        "required": ["requiredField"]
                                    }
                                }
                            }
                        }
                    }
                }
            }
        })
    }

    #[test]
    fn test_collect_parameters() {
        let openapi = create_test_openapi();
        let generator = CodeGenerator::new(openapi.clone());
        let operation = openapi["paths"]["/test"]["get"].clone();

        let params = generator.collect_parameters(&operation);

        assert!(params.contains(&"queryParam: z.string().optional()".to_string()));
        assert!(params.contains(&"requiredField: z.string()".to_string()));
        assert!(params.contains(&"optionalField: z.number().optional()".to_string()));
    }

    #[test]
    fn test_generate_tool() {
        let openapi = create_test_openapi();
        let generator = CodeGenerator::new(openapi.clone());
        let operation = openapi["paths"]["/test"]["get"].clone();

        let mut code = String::new();
        generator.generate_tool(&mut code, "/test", "get", &operation);

        assert!(code.contains("testOperation"));
        assert!(code.contains("const search = new URLSearchParams()"));
        assert!(code.contains("method: \"GET\""));
    }

    #[test]
    fn test_add_imports() {
        let openapi = create_test_openapi();
        let generator = CodeGenerator::new(openapi);

        let mut code = String::new();
        generator.add_imports(&mut code);

        assert!(code.contains("import { McpServer }"));
        assert!(code.contains("import { StdioServerTransport }"));
        assert!(code.contains("import dotenv"));
        assert!(code.contains("import { z }"));
        assert!(code.contains("const server = new McpServer"));
    }

    #[test]
    fn test_add_server_connection() {
        let openapi = create_test_openapi();
        let generator = CodeGenerator::new(openapi);

        let mut code = String::new();
        generator.add_server_connection(&mut code);

        assert!(code.contains("const transport = new StdioServerTransport()"));
        assert!(code.contains("await server.connect(transport)"));
    }

    #[test]
    fn test_generate() {
        let openapi = create_test_openapi();
        let generator = CodeGenerator::new(openapi);

        let code = generator.generate();

        assert!(code.contains("import { McpServer }"));
        assert!(code.contains("testOperation"));
        assert!(code.contains("await server.connect(transport)"));
    }
}
