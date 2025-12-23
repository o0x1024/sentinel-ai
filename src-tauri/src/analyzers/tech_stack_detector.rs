//! Technology stack detection from HTTP responses

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use super::website_analyzer::ProxyRequest;

/// Technology stack information
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TechStack {
    /// Web server (e.g., nginx, apache, IIS)
    pub server: Option<String>,
    /// Backend framework (e.g., Django, Spring, Laravel)
    pub framework: Option<String>,
    /// Database hints (e.g., MySQL, PostgreSQL, MongoDB)
    pub database: Option<String>,
    /// Programming language (e.g., PHP, Python, Java, Node.js)
    pub language: Option<String>,
    /// Additional technologies detected
    pub others: Vec<String>,
}

/// Technology stack detector
pub struct TechStackDetector;

impl TechStackDetector {
    pub fn new() -> Self {
        Self
    }

    /// Detect technology stack from requests
    pub fn detect(&self, requests: &[ProxyRequest]) -> TechStack {
        let mut tech_stack = TechStack::default();
        
        // Collect all headers
        let mut all_headers: Vec<HashMap<String, String>> = Vec::new();
        let mut all_bodies: Vec<String> = Vec::new();
        
        for req in requests {
            if let Some(ref headers_str) = req.response_headers {
                if let Ok(headers) = serde_json::from_str::<HashMap<String, String>>(headers_str) {
                    all_headers.push(headers);
                }
            }
            if let Some(ref body) = req.response_body {
                all_bodies.push(body.clone());
            }
        }

        // Detect server
        tech_stack.server = self.detect_server(&all_headers);
        
        // Detect framework
        tech_stack.framework = self.detect_framework(&all_headers, &all_bodies);
        
        // Detect database
        tech_stack.database = self.detect_database(&all_bodies);
        
        // Detect language
        tech_stack.language = self.detect_language(&all_headers, &all_bodies);
        
        // Detect other technologies
        tech_stack.others = self.detect_others(&all_headers, &all_bodies);

        tech_stack
    }

    /// Detect web server
    fn detect_server(&self, headers: &[HashMap<String, String>]) -> Option<String> {
        for header_map in headers {
            for (key, value) in header_map {
                if key.to_lowercase() == "server" {
                    let value_lower = value.to_lowercase();
                    
                    if value_lower.contains("nginx") {
                        return Some("nginx".to_string());
                    } else if value_lower.contains("apache") {
                        return Some("Apache".to_string());
                    } else if value_lower.contains("iis") || value_lower.contains("microsoft") {
                        return Some("IIS".to_string());
                    } else if value_lower.contains("cloudflare") {
                        return Some("Cloudflare".to_string());
                    } else if value_lower.contains("tengine") {
                        return Some("Tengine".to_string());
                    } else if value_lower.contains("openresty") {
                        return Some("OpenResty".to_string());
                    } else if value_lower.contains("caddy") {
                        return Some("Caddy".to_string());
                    } else {
                        return Some(value.clone());
                    }
                }
            }
        }
        None
    }

    /// Detect backend framework
    fn detect_framework(&self, headers: &[HashMap<String, String>], bodies: &[String]) -> Option<String> {
        // Check headers first
        for header_map in headers {
            for (key, value) in header_map {
                let key_lower = key.to_lowercase();
                let value_lower = value.to_lowercase();

                // Framework-specific headers
                if key_lower.contains("x-powered-by") {
                    if value_lower.contains("express") {
                        return Some("Express.js".to_string());
                    } else if value_lower.contains("php") {
                        return Some("PHP".to_string());
                    } else if value_lower.contains("asp.net") {
                        return Some("ASP.NET".to_string());
                    }
                }

                if key_lower.contains("x-aspnet-version") {
                    return Some("ASP.NET".to_string());
                }

                if key_lower.contains("x-django") || value_lower.contains("django") {
                    return Some("Django".to_string());
                }

                if key_lower.contains("x-rails") || value_lower.contains("rails") {
                    return Some("Ruby on Rails".to_string());
                }

                if key_lower.contains("x-laravel") || value_lower.contains("laravel") {
                    return Some("Laravel".to_string());
                }
            }
        }

        // Check response bodies for framework signatures
        for body in bodies {
            let body_lower = body.to_lowercase();

            if body_lower.contains("django") {
                return Some("Django".to_string());
            } else if body_lower.contains("spring framework") || body_lower.contains("springframework") {
                return Some("Spring".to_string());
            } else if body_lower.contains("laravel") {
                return Some("Laravel".to_string());
            } else if body_lower.contains("flask") {
                return Some("Flask".to_string());
            } else if body_lower.contains("fastapi") {
                return Some("FastAPI".to_string());
            } else if body_lower.contains("next.js") || body_lower.contains("nextjs") {
                return Some("Next.js".to_string());
            } else if body_lower.contains("nuxt") {
                return Some("Nuxt.js".to_string());
            }
        }

        None
    }

    /// Detect database from error messages
    fn detect_database(&self, bodies: &[String]) -> Option<String> {
        for body in bodies {
            let body_lower = body.to_lowercase();

            // MySQL
            if body_lower.contains("mysql") || 
               body_lower.contains("you have an error in your sql syntax") ||
               body_lower.contains("mysqli") {
                return Some("MySQL".to_string());
            }

            // PostgreSQL
            if body_lower.contains("postgresql") || 
               body_lower.contains("psql") ||
               body_lower.contains("pg_") {
                return Some("PostgreSQL".to_string());
            }

            // MongoDB
            if body_lower.contains("mongodb") || 
               body_lower.contains("mongo error") {
                return Some("MongoDB".to_string());
            }

            // Oracle
            if body_lower.contains("ora-") || 
               body_lower.contains("oracle") {
                return Some("Oracle".to_string());
            }

            // Microsoft SQL Server
            if body_lower.contains("mssql") || 
               body_lower.contains("sql server") ||
               body_lower.contains("microsoft ole db provider") {
                return Some("MSSQL".to_string());
            }

            // SQLite
            if body_lower.contains("sqlite") {
                return Some("SQLite".to_string());
            }

            // Redis
            if body_lower.contains("redis") {
                return Some("Redis".to_string());
            }
        }

        None
    }

    /// Detect programming language
    fn detect_language(&self, headers: &[HashMap<String, String>], bodies: &[String]) -> Option<String> {
        // Check headers
        for header_map in headers {
            for (key, value) in header_map {
                let key_lower = key.to_lowercase();
                let value_lower = value.to_lowercase();

                if key_lower.contains("x-powered-by") {
                    if value_lower.contains("php") {
                        return Some("PHP".to_string());
                    } else if value_lower.contains("asp.net") {
                        return Some("C#".to_string());
                    }
                }

                // Session cookies can reveal technology
                if key_lower == "set-cookie" {
                    if value_lower.contains("phpsessid") {
                        return Some("PHP".to_string());
                    } else if value_lower.contains("jsessionid") {
                        return Some("Java".to_string());
                    } else if value_lower.contains("asp.net") {
                        return Some("C#".to_string());
                    }
                }
            }
        }

        // Check bodies for error messages
        for body in bodies {
            let body_lower = body.to_lowercase();

            if body_lower.contains("fatal error") && body_lower.contains(".php") {
                return Some("PHP".to_string());
            } else if body_lower.contains("traceback") && body_lower.contains(".py") {
                return Some("Python".to_string());
            } else if body_lower.contains("at java.") || body_lower.contains("exception in thread") {
                return Some("Java".to_string());
            } else if body_lower.contains("error") && body_lower.contains(".rb") {
                return Some("Ruby".to_string());
            } else if body_lower.contains("node.js") || body_lower.contains("at node:") {
                return Some("Node.js".to_string());
            } else if body_lower.contains(".cs") && body_lower.contains("system.") {
                return Some("C#".to_string());
            } else if body_lower.contains("go routine") || body_lower.contains("goroutine") {
                return Some("Go".to_string());
            }
        }

        None
    }

    /// Detect other technologies
    fn detect_others(&self, headers: &[HashMap<String, String>], bodies: &[String]) -> Vec<String> {
        let mut others = Vec::new();

        // Check headers
        for header_map in headers {
            for (key, value) in header_map {
                let key_lower = key.to_lowercase();
                let value_lower = value.to_lowercase();

                // CDN detection
                if (key_lower.contains("cf-ray") || value_lower.contains("cloudflare"))
                    && !others.contains(&"Cloudflare CDN".to_string()) {
                        others.push("Cloudflare CDN".to_string());
                    }

                if (key_lower.contains("x-amz") || value_lower.contains("amazons3"))
                    && !others.contains(&"AWS".to_string()) {
                        others.push("AWS".to_string());
                    }

                // WordPress
                if value_lower.contains("wordpress")
                    && !others.contains(&"WordPress".to_string()) {
                        others.push("WordPress".to_string());
                    }

                // Load balancer
                if (key_lower.contains("x-lb") || value_lower.contains("load-balancer"))
                    && !others.contains(&"Load Balancer".to_string()) {
                        others.push("Load Balancer".to_string());
                    }
            }
        }

        // Check bodies
        for body in bodies {
            let body_lower = body.to_lowercase();

            if (body_lower.contains("wp-content") || body_lower.contains("wp-includes"))
                && !others.contains(&"WordPress".to_string()) {
                    others.push("WordPress".to_string());
                }

            if body_lower.contains("jquery")
                && !others.contains(&"jQuery".to_string()) {
                    others.push("jQuery".to_string());
                }

            if (body_lower.contains("react") || body_lower.contains("reactjs"))
                && !others.contains(&"React".to_string()) {
                    others.push("React".to_string());
                }

            if (body_lower.contains("vue") || body_lower.contains("vuejs"))
                && !others.contains(&"Vue.js".to_string()) {
                    others.push("Vue.js".to_string());
                }

            if body_lower.contains("angular")
                && !others.contains(&"Angular".to_string()) {
                    others.push("Angular".to_string());
                }
        }

        others
    }
}

impl Default for TechStackDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_server() {
        let detector = TechStackDetector::new();
        let mut headers = HashMap::new();
        headers.insert("server".to_string(), "nginx/1.18.0".to_string());
        
        let result = detector.detect_server(&[headers]);
        assert_eq!(result, Some("nginx".to_string()));
    }

    #[test]
    fn test_detect_framework_from_header() {
        let detector = TechStackDetector::new();
        let mut headers = HashMap::new();
        headers.insert("X-Powered-By".to_string(), "Express".to_string());
        
        let result = detector.detect_framework(&[headers], &[]);
        assert_eq!(result, Some("Express.js".to_string()));
    }

    #[test]
    fn test_detect_database() {
        let detector = TechStackDetector::new();
        let body = "Error: MySQL syntax error at line 1".to_string();
        
        let result = detector.detect_database(&[body]);
        assert_eq!(result, Some("MySQL".to_string()));
    }
}

