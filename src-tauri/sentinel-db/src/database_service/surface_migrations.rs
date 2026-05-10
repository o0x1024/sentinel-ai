use crate::database_service::connection_manager::DatabasePool;
use crate::database_service::sqlx_compat::PgPool;
use anyhow::Result;
use sqlx::Row;
use tracing::info;

pub struct SurfaceGraphMigration;

impl SurfaceGraphMigration {
    pub async fn apply(pool: &PgPool) -> Result<()> {
        Self::apply_postgres(pool).await
    }

    pub async fn apply_runtime(pool: &DatabasePool) -> Result<()> {
        info!("Applying surface graph migration...");

        let table_sql = [
            r#"CREATE TABLE IF NOT EXISTS surface_assets (
                id TEXT PRIMARY KEY,
                program_id TEXT NOT NULL,
                asset_type TEXT NOT NULL,
                asset_name TEXT NOT NULL,
                display_name TEXT,
                description TEXT,
                org_id TEXT,
                business_unit TEXT,
                project TEXT,
                owner TEXT,
                maintainer TEXT,
                contact TEXT,
                env TEXT,
                internet_exposure TEXT,
                criticality TEXT,
                data_level TEXT,
                source TEXT,
                first_seen_at TIMESTAMPTZ NOT NULL,
                last_seen_at TIMESTAMPTZ NOT NULL,
                last_verified_at TIMESTAMPTZ,
                discovery_task_id TEXT,
                status TEXT NOT NULL,
                alive_status TEXT,
                confidence_score DOUBLE PRECISION,
                fingerprint_confidence DOUBLE PRECISION,
                risk_score DOUBLE PRECISION,
                risk_level TEXT,
                vulnerabilities_count INTEGER DEFAULT 0,
                weak_password_flag BOOLEAN DEFAULT FALSE,
                expired_cert_flag BOOLEAN DEFAULT FALSE,
                exposed_to_internet_flag BOOLEAN DEFAULT FALSE,
                viewed_at TIMESTAMPTZ,
                viewed_by TEXT,
                metadata_json TEXT,
                created_at TIMESTAMPTZ NOT NULL,
                updated_at TIMESTAMPTZ NOT NULL,
                created_by TEXT,
                updated_by TEXT
            )"#,
            r#"CREATE TABLE IF NOT EXISTS surface_org_assets (
                asset_id TEXT PRIMARY KEY,
                org_name TEXT NOT NULL,
                org_short_name TEXT,
                parent_org_id TEXT,
                business_line TEXT,
                importance_level TEXT,
                notes TEXT
            )"#,
            r#"CREATE TABLE IF NOT EXISTS surface_domain_assets (
                asset_id TEXT PRIMARY KEY,
                fqdn TEXT NOT NULL,
                main_domain TEXT,
                root_domain TEXT,
                subdomain_level INTEGER,
                record_type TEXT,
                record_value TEXT,
                ttl INTEGER,
                registrar TEXT,
                registered_at TIMESTAMPTZ,
                expires_at TIMESTAMPTZ,
                whois_json TEXT,
                wildcard_enabled BOOLEAN DEFAULT FALSE,
                dnssec_enabled BOOLEAN DEFAULT FALSE
            )"#,
            r#"CREATE TABLE IF NOT EXISTS surface_ip_assets (
                asset_id TEXT PRIMARY KEY,
                ip_address TEXT NOT NULL,
                ip_version TEXT,
                cidr TEXT,
                asn INTEGER,
                bgp_prefix TEXT,
                country TEXT,
                region TEXT,
                city TEXT,
                isp TEXT,
                reverse_dns TEXT,
                cloud_provider TEXT,
                network_boundary_type TEXT
            )"#,
            r#"CREATE TABLE IF NOT EXISTS surface_host_assets (
                asset_id TEXT PRIMARY KEY,
                hostname TEXT NOT NULL,
                fqdn TEXT,
                ip_addresses_json TEXT,
                operating_system TEXT,
                device_type TEXT,
                cloud_instance_id TEXT,
                region_or_datacenter TEXT,
                vpc_or_subnet TEXT,
                mac_address TEXT,
                agent_status TEXT,
                labels_json TEXT,
                lifecycle_status TEXT,
                last_online_at TIMESTAMPTZ
            )"#,
            r#"CREATE TABLE IF NOT EXISTS surface_port_assets (
                asset_id TEXT PRIMARY KEY,
                host_asset_id TEXT,
                ip_address TEXT,
                port_number INTEGER NOT NULL,
                transport_protocol TEXT NOT NULL,
                port_state TEXT,
                scan_source TEXT,
                related_service_asset_id TEXT
            )"#,
            r#"CREATE TABLE IF NOT EXISTS surface_service_assets (
                asset_id TEXT PRIMARY KEY,
                host_asset_id TEXT,
                ip_address TEXT,
                port_number INTEGER,
                transport_protocol TEXT,
                protocol_name TEXT,
                application_service_name TEXT,
                banner TEXT,
                product_name TEXT,
                vendor TEXT,
                version TEXT,
                middleware_type TEXT,
                component_fingerprint TEXT,
                auth_type TEXT,
                login_required BOOLEAN DEFAULT FALSE,
                weak_password_risk BOOLEAN DEFAULT FALSE,
                encrypted_transport BOOLEAN DEFAULT FALSE,
                related_cves_json TEXT,
                related_domain_asset_id TEXT,
                system_asset_id TEXT
            )"#,
            r#"CREATE TABLE IF NOT EXISTS surface_web_assets (
                asset_id TEXT PRIMARY KEY,
                service_asset_id TEXT,
                canonical_url TEXT NOT NULL,
                scheme TEXT,
                domain_asset_id TEXT,
                ip_address TEXT,
                port_number INTEGER,
                site_title TEXT,
                http_status_code INTEGER,
                server_header TEXT,
                response_headers_json TEXT,
                page_fingerprint TEXT,
                favicon_hash TEXT,
                screenshot_path TEXT,
                framework TEXT,
                cms TEXT,
                waf_flag BOOLEAN DEFAULT FALSE,
                cdn_flag BOOLEAN DEFAULT FALSE,
                login_flag BOOLEAN DEFAULT FALSE,
                api_flag BOOLEAN DEFAULT FALSE,
                openapi_url TEXT,
                business_type TEXT,
                language TEXT,
                filing_info TEXT,
                content_summary TEXT,
                sensitive_path_results_json TEXT,
                last_accessed_at TIMESTAMPTZ
            )"#,
            r#"CREATE TABLE IF NOT EXISTS surface_cert_assets (
                asset_id TEXT PRIMARY KEY,
                sha256 TEXT NOT NULL,
                sha1 TEXT,
                serial_number TEXT,
                public_key_algorithm TEXT,
                signature_algorithm TEXT,
                issuer TEXT,
                subject TEXT,
                san_list_json TEXT,
                valid_from TIMESTAMPTZ,
                valid_to TIMESTAMPTZ,
                self_signed_flag BOOLEAN DEFAULT FALSE,
                related_domains_json TEXT,
                related_ips_json TEXT,
                certificate_chain_json TEXT,
                risk_status TEXT
            )"#,
            r#"CREATE TABLE IF NOT EXISTS surface_relations (
                id TEXT PRIMARY KEY,
                program_id TEXT NOT NULL,
                from_asset_id TEXT NOT NULL,
                to_asset_id TEXT NOT NULL,
                relation_type TEXT NOT NULL,
                source TEXT,
                confidence_score DOUBLE PRECISION,
                evidence_id TEXT,
                first_seen_at TIMESTAMPTZ NOT NULL,
                last_seen_at TIMESTAMPTZ NOT NULL,
                active BOOLEAN DEFAULT TRUE,
                metadata_json TEXT,
                created_at TIMESTAMPTZ NOT NULL,
                updated_at TIMESTAMPTZ NOT NULL
            )"#,
            r#"CREATE TABLE IF NOT EXISTS surface_fingerprints (
                id TEXT PRIMARY KEY,
                program_id TEXT NOT NULL,
                asset_id TEXT NOT NULL,
                fingerprint_type TEXT NOT NULL,
                fingerprint_key TEXT,
                fingerprint_value TEXT NOT NULL,
                rule_id TEXT,
                rule_word TEXT,
                rule_name TEXT,
                normalized_product TEXT,
                normalized_vendor TEXT,
                normalized_category TEXT,
                normalized_family TEXT,
                version TEXT,
                is_primary BOOLEAN DEFAULT FALSE,
                match_source_part TEXT,
                confidence_score DOUBLE PRECISION,
                source TEXT,
                observed_at TIMESTAMPTZ NOT NULL,
                metadata_json TEXT
            )"#,
            r#"CREATE TABLE IF NOT EXISTS surface_asset_classifications (
                id TEXT PRIMARY KEY,
                program_id TEXT NOT NULL,
                asset_id TEXT NOT NULL,
                primary_category TEXT NOT NULL,
                primary_product TEXT NOT NULL,
                primary_vendor TEXT,
                primary_family TEXT,
                rule_id TEXT,
                rule_name TEXT,
                confidence_score DOUBLE PRECISION,
                source TEXT,
                classified_at TIMESTAMPTZ NOT NULL,
                metadata_json TEXT
            )"#,
            r#"CREATE TABLE IF NOT EXISTS surface_evidence (
                id TEXT PRIMARY KEY,
                program_id TEXT NOT NULL,
                asset_id TEXT,
                evidence_type TEXT NOT NULL,
                title TEXT,
                content_text TEXT,
                content_path TEXT,
                content_json TEXT,
                collected_at TIMESTAMPTZ NOT NULL,
                collected_by TEXT,
                probe_node TEXT,
                metadata_json TEXT
            )"#,
            r#"CREATE TABLE IF NOT EXISTS surface_change_logs (
                id TEXT PRIMARY KEY,
                program_id TEXT NOT NULL,
                asset_id TEXT,
                relation_id TEXT,
                change_type TEXT NOT NULL,
                old_value_json TEXT,
                new_value_json TEXT,
                summary TEXT NOT NULL,
                detected_at TIMESTAMPTZ NOT NULL,
                source_run_id TEXT,
                risk_delta DOUBLE PRECISION,
                metadata_json TEXT
            )"#,
            r#"CREATE TABLE IF NOT EXISTS surface_discovery_runs (
                id TEXT PRIMARY KEY,
                program_id TEXT NOT NULL,
                trigger_source TEXT NOT NULL,
                schedule_id TEXT,
                workflow_id TEXT,
                workflow_template_id TEXT,
                plugin_id TEXT,
                status TEXT NOT NULL,
                seed_count INTEGER DEFAULT 0,
                observation_count INTEGER DEFAULT 0,
                imported_asset_count INTEGER DEFAULT 0,
                changed_asset_count INTEGER DEFAULT 0,
                error_message TEXT,
                started_at TIMESTAMPTZ NOT NULL,
                completed_at TIMESTAMPTZ,
                metadata_json TEXT
            )"#,
            r#"CREATE TABLE IF NOT EXISTS surface_observations (
                id TEXT PRIMARY KEY,
                run_id TEXT NOT NULL,
                program_id TEXT NOT NULL,
                artifact_type TEXT NOT NULL,
                object_key TEXT,
                payload_json TEXT NOT NULL,
                source_plugin TEXT,
                confidence_score DOUBLE PRECISION,
                observed_at TIMESTAMPTZ NOT NULL,
                normalized BOOLEAN DEFAULT FALSE,
                metadata_json TEXT
            )"#,
            r#"CREATE TABLE IF NOT EXISTS surface_seeds (
                id TEXT PRIMARY KEY,
                program_id TEXT NOT NULL,
                seed_type TEXT NOT NULL,
                seed_value TEXT NOT NULL,
                status TEXT NOT NULL,
                source TEXT,
                confidence_score DOUBLE PRECISION,
                last_run_at TIMESTAMPTZ,
                metadata_json TEXT,
                created_at TIMESTAMPTZ NOT NULL,
                updated_at TIMESTAMPTZ NOT NULL
            )"#,
            r#"CREATE TABLE IF NOT EXISTS surface_seed_candidates (
                id TEXT PRIMARY KEY,
                program_id TEXT NOT NULL,
                seed_type TEXT NOT NULL,
                seed_value TEXT NOT NULL,
                status TEXT NOT NULL,
                source_asset_id TEXT NOT NULL,
                source_asset_type TEXT NOT NULL,
                source_detail_key TEXT NOT NULL,
                source_display_value TEXT NOT NULL,
                source_canonical_url TEXT,
                confidence_score DOUBLE PRECISION,
                observed_at TIMESTAMPTZ NOT NULL,
                reviewed_at TIMESTAMPTZ,
                metadata_json TEXT,
                created_at TIMESTAMPTZ NOT NULL,
                updated_at TIMESTAMPTZ NOT NULL
            )"#,
        ];

        let index_sql = [
            "CREATE INDEX IF NOT EXISTS idx_surface_assets_program ON surface_assets(program_id)",
            "CREATE INDEX IF NOT EXISTS idx_surface_assets_type ON surface_assets(asset_type)",
            "CREATE INDEX IF NOT EXISTS idx_surface_assets_status ON surface_assets(status)",
            "CREATE INDEX IF NOT EXISTS idx_surface_assets_last_seen ON surface_assets(last_seen_at DESC)",
            "CREATE INDEX IF NOT EXISTS idx_surface_assets_discovery_task ON surface_assets(discovery_task_id)",
            "CREATE INDEX IF NOT EXISTS idx_surface_assets_risk ON surface_assets(risk_score DESC)",
            "CREATE INDEX IF NOT EXISTS idx_surface_assets_viewed ON surface_assets(viewed_at)",
            "CREATE INDEX IF NOT EXISTS idx_surface_assets_program_viewed_type ON surface_assets(program_id, viewed_at, asset_type)",
            "CREATE INDEX IF NOT EXISTS idx_surface_assets_program_type_status_seen ON surface_assets(program_id, asset_type, status, last_seen_at DESC, id)",
            "CREATE INDEX IF NOT EXISTS idx_surface_domain_assets_fqdn ON surface_domain_assets(fqdn)",
            "CREATE INDEX IF NOT EXISTS idx_surface_domain_assets_root ON surface_domain_assets(root_domain)",
            "CREATE INDEX IF NOT EXISTS idx_surface_ip_assets_ip ON surface_ip_assets(ip_address)",
            "CREATE INDEX IF NOT EXISTS idx_surface_ip_assets_cidr ON surface_ip_assets(cidr)",
            "CREATE INDEX IF NOT EXISTS idx_surface_host_assets_hostname ON surface_host_assets(hostname)",
            "CREATE INDEX IF NOT EXISTS idx_surface_port_assets_host_port ON surface_port_assets(host_asset_id, port_number, transport_protocol)",
            "CREATE INDEX IF NOT EXISTS idx_surface_service_assets_host_port ON surface_service_assets(host_asset_id, port_number, transport_protocol)",
            "CREATE INDEX IF NOT EXISTS idx_surface_web_assets_url ON surface_web_assets(canonical_url)",
            "CREATE INDEX IF NOT EXISTS idx_surface_cert_assets_sha256 ON surface_cert_assets(sha256)",
            "CREATE INDEX IF NOT EXISTS idx_surface_relations_program ON surface_relations(program_id)",
            "CREATE INDEX IF NOT EXISTS idx_surface_relations_from_asset ON surface_relations(from_asset_id)",
            "CREATE INDEX IF NOT EXISTS idx_surface_relations_to_asset ON surface_relations(to_asset_id)",
            "CREATE INDEX IF NOT EXISTS idx_surface_relations_from_to ON surface_relations(from_asset_id, to_asset_id)",
            "CREATE INDEX IF NOT EXISTS idx_surface_relations_program_from_to_type ON surface_relations(program_id, from_asset_id, to_asset_id, relation_type)",
            "CREATE INDEX IF NOT EXISTS idx_surface_relations_type ON surface_relations(relation_type)",
            "CREATE INDEX IF NOT EXISTS idx_surface_fingerprints_asset ON surface_fingerprints(asset_id)",
            "CREATE INDEX IF NOT EXISTS idx_surface_fingerprints_program_category ON surface_fingerprints(program_id, normalized_category)",
            "CREATE INDEX IF NOT EXISTS idx_surface_fingerprints_program_product ON surface_fingerprints(program_id, normalized_product)",
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_surface_asset_classifications_asset ON surface_asset_classifications(asset_id)",
            "CREATE INDEX IF NOT EXISTS idx_surface_asset_classifications_program_category ON surface_asset_classifications(program_id, primary_category)",
            "CREATE INDEX IF NOT EXISTS idx_surface_asset_classifications_program_product ON surface_asset_classifications(program_id, primary_product)",
            "CREATE INDEX IF NOT EXISTS idx_surface_evidence_asset ON surface_evidence(asset_id)",
            "CREATE INDEX IF NOT EXISTS idx_surface_change_logs_asset ON surface_change_logs(asset_id)",
            "CREATE INDEX IF NOT EXISTS idx_surface_change_logs_program ON surface_change_logs(program_id)",
            "CREATE INDEX IF NOT EXISTS idx_surface_change_logs_source_run ON surface_change_logs(source_run_id)",
            "CREATE INDEX IF NOT EXISTS idx_surface_change_logs_detected ON surface_change_logs(detected_at DESC)",
            "CREATE INDEX IF NOT EXISTS idx_surface_discovery_runs_program ON surface_discovery_runs(program_id)",
            "CREATE INDEX IF NOT EXISTS idx_surface_discovery_runs_started ON surface_discovery_runs(started_at DESC)",
            "CREATE INDEX IF NOT EXISTS idx_surface_observations_run ON surface_observations(run_id)",
            "CREATE INDEX IF NOT EXISTS idx_surface_observations_type ON surface_observations(artifact_type)",
            "CREATE INDEX IF NOT EXISTS idx_surface_observations_plugin_artifact_target ON surface_observations(source_plugin, artifact_type, program_id, object_key, observed_at DESC)",
            "CREATE INDEX IF NOT EXISTS idx_surface_seeds_program ON surface_seeds(program_id)",
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_surface_seed_candidates_identity ON surface_seed_candidates(program_id, seed_type, seed_value, source_asset_id, source_detail_key)",
            "CREATE INDEX IF NOT EXISTS idx_surface_seed_candidates_program_status ON surface_seed_candidates(program_id, status, observed_at DESC)",
        ];

        for sql in table_sql {
            match pool {
                DatabasePool::PostgreSQL(pg) => {
                    sqlx::query(sql).execute(pg).await?;
                }
                DatabasePool::SQLite(sqlite) => {
                    sqlx::query(sql).execute(sqlite).await?;
                }
                DatabasePool::MySQL(mysql) => {
                    sqlx::query(sql).execute(mysql).await?;
                }
            }
        }

        let surface_asset_alter_columns = [("viewed_at", "TIMESTAMPTZ"), ("viewed_by", "TEXT")];

        for (column, column_type) in surface_asset_alter_columns {
            Self::add_column_if_not_exists_runtime(pool, "surface_assets", column, column_type)
                .await?;
        }

        let alter_columns = [
            ("rule_id", "TEXT"),
            ("rule_word", "TEXT"),
            ("rule_name", "TEXT"),
            ("normalized_product", "TEXT"),
            ("normalized_vendor", "TEXT"),
            ("normalized_category", "TEXT"),
            ("normalized_family", "TEXT"),
            ("version", "TEXT"),
            ("is_primary", "BOOLEAN DEFAULT FALSE"),
            ("match_source_part", "TEXT"),
        ];

        for (column, column_type) in alter_columns {
            Self::add_column_if_not_exists_runtime(
                pool,
                "surface_fingerprints",
                column,
                column_type,
            )
            .await?;
        }

        for sql in index_sql {
            match pool {
                DatabasePool::PostgreSQL(pg) => {
                    sqlx::query(sql).execute(pg).await?;
                }
                DatabasePool::SQLite(sqlite) => {
                    sqlx::query(sql).execute(sqlite).await?;
                }
                DatabasePool::MySQL(mysql) => {
                    sqlx::query(sql).execute(mysql).await?;
                }
            }
        }

        info!("Surface graph migration completed successfully");
        Ok(())
    }

    async fn apply_postgres(pool: &PgPool) -> Result<()> {
        info!("Applying surface graph migration...");

        let table_sql = [
            r#"CREATE TABLE IF NOT EXISTS surface_assets (
                id TEXT PRIMARY KEY,
                program_id TEXT NOT NULL,
                asset_type TEXT NOT NULL,
                asset_name TEXT NOT NULL,
                display_name TEXT,
                description TEXT,
                org_id TEXT,
                business_unit TEXT,
                project TEXT,
                owner TEXT,
                maintainer TEXT,
                contact TEXT,
                env TEXT,
                internet_exposure TEXT,
                criticality TEXT,
                data_level TEXT,
                source TEXT,
                first_seen_at TIMESTAMPTZ NOT NULL,
                last_seen_at TIMESTAMPTZ NOT NULL,
                last_verified_at TIMESTAMPTZ,
                discovery_task_id TEXT,
                status TEXT NOT NULL,
                alive_status TEXT,
                confidence_score DOUBLE PRECISION,
                fingerprint_confidence DOUBLE PRECISION,
                risk_score DOUBLE PRECISION,
                risk_level TEXT,
                vulnerabilities_count INTEGER DEFAULT 0,
                weak_password_flag BOOLEAN DEFAULT FALSE,
                expired_cert_flag BOOLEAN DEFAULT FALSE,
                exposed_to_internet_flag BOOLEAN DEFAULT FALSE,
                viewed_at TIMESTAMPTZ,
                viewed_by TEXT,
                metadata_json TEXT,
                created_at TIMESTAMPTZ NOT NULL,
                updated_at TIMESTAMPTZ NOT NULL,
                created_by TEXT,
                updated_by TEXT
            )"#,
            r#"CREATE TABLE IF NOT EXISTS surface_org_assets (
                asset_id TEXT PRIMARY KEY,
                org_name TEXT NOT NULL,
                org_short_name TEXT,
                parent_org_id TEXT,
                business_line TEXT,
                importance_level TEXT,
                notes TEXT
            )"#,
            r#"CREATE TABLE IF NOT EXISTS surface_domain_assets (
                asset_id TEXT PRIMARY KEY,
                fqdn TEXT NOT NULL,
                main_domain TEXT,
                root_domain TEXT,
                subdomain_level INTEGER,
                record_type TEXT,
                record_value TEXT,
                ttl INTEGER,
                registrar TEXT,
                registered_at TIMESTAMPTZ,
                expires_at TIMESTAMPTZ,
                whois_json TEXT,
                wildcard_enabled BOOLEAN DEFAULT FALSE,
                dnssec_enabled BOOLEAN DEFAULT FALSE
            )"#,
            r#"CREATE TABLE IF NOT EXISTS surface_ip_assets (
                asset_id TEXT PRIMARY KEY,
                ip_address TEXT NOT NULL,
                ip_version TEXT,
                cidr TEXT,
                asn INTEGER,
                bgp_prefix TEXT,
                country TEXT,
                region TEXT,
                city TEXT,
                isp TEXT,
                reverse_dns TEXT,
                cloud_provider TEXT,
                network_boundary_type TEXT
            )"#,
            r#"CREATE TABLE IF NOT EXISTS surface_host_assets (
                asset_id TEXT PRIMARY KEY,
                hostname TEXT NOT NULL,
                fqdn TEXT,
                ip_addresses_json TEXT,
                operating_system TEXT,
                device_type TEXT,
                cloud_instance_id TEXT,
                region_or_datacenter TEXT,
                vpc_or_subnet TEXT,
                mac_address TEXT,
                agent_status TEXT,
                labels_json TEXT,
                lifecycle_status TEXT,
                last_online_at TIMESTAMPTZ
            )"#,
            r#"CREATE TABLE IF NOT EXISTS surface_port_assets (
                asset_id TEXT PRIMARY KEY,
                host_asset_id TEXT,
                ip_address TEXT,
                port_number INTEGER NOT NULL,
                transport_protocol TEXT NOT NULL,
                port_state TEXT,
                scan_source TEXT,
                related_service_asset_id TEXT
            )"#,
            r#"CREATE TABLE IF NOT EXISTS surface_service_assets (
                asset_id TEXT PRIMARY KEY,
                host_asset_id TEXT,
                ip_address TEXT,
                port_number INTEGER,
                transport_protocol TEXT,
                protocol_name TEXT,
                application_service_name TEXT,
                banner TEXT,
                product_name TEXT,
                vendor TEXT,
                version TEXT,
                middleware_type TEXT,
                component_fingerprint TEXT,
                auth_type TEXT,
                login_required BOOLEAN DEFAULT FALSE,
                weak_password_risk BOOLEAN DEFAULT FALSE,
                encrypted_transport BOOLEAN DEFAULT FALSE,
                related_cves_json TEXT,
                related_domain_asset_id TEXT,
                system_asset_id TEXT
            )"#,
            r#"CREATE TABLE IF NOT EXISTS surface_web_assets (
                asset_id TEXT PRIMARY KEY,
                service_asset_id TEXT,
                canonical_url TEXT NOT NULL,
                scheme TEXT,
                domain_asset_id TEXT,
                ip_address TEXT,
                port_number INTEGER,
                site_title TEXT,
                http_status_code INTEGER,
                server_header TEXT,
                response_headers_json TEXT,
                page_fingerprint TEXT,
                favicon_hash TEXT,
                screenshot_path TEXT,
                framework TEXT,
                cms TEXT,
                waf_flag BOOLEAN DEFAULT FALSE,
                cdn_flag BOOLEAN DEFAULT FALSE,
                login_flag BOOLEAN DEFAULT FALSE,
                api_flag BOOLEAN DEFAULT FALSE,
                openapi_url TEXT,
                business_type TEXT,
                language TEXT,
                filing_info TEXT,
                content_summary TEXT,
                sensitive_path_results_json TEXT,
                last_accessed_at TIMESTAMPTZ
            )"#,
            r#"CREATE TABLE IF NOT EXISTS surface_cert_assets (
                asset_id TEXT PRIMARY KEY,
                sha256 TEXT NOT NULL,
                sha1 TEXT,
                serial_number TEXT,
                public_key_algorithm TEXT,
                signature_algorithm TEXT,
                issuer TEXT,
                subject TEXT,
                san_list_json TEXT,
                valid_from TIMESTAMPTZ,
                valid_to TIMESTAMPTZ,
                self_signed_flag BOOLEAN DEFAULT FALSE,
                related_domains_json TEXT,
                related_ips_json TEXT,
                certificate_chain_json TEXT,
                risk_status TEXT
            )"#,
            r#"CREATE TABLE IF NOT EXISTS surface_relations (
                id TEXT PRIMARY KEY,
                program_id TEXT NOT NULL,
                from_asset_id TEXT NOT NULL,
                to_asset_id TEXT NOT NULL,
                relation_type TEXT NOT NULL,
                source TEXT,
                confidence_score DOUBLE PRECISION,
                evidence_id TEXT,
                first_seen_at TIMESTAMPTZ NOT NULL,
                last_seen_at TIMESTAMPTZ NOT NULL,
                active BOOLEAN DEFAULT TRUE,
                metadata_json TEXT,
                created_at TIMESTAMPTZ NOT NULL,
                updated_at TIMESTAMPTZ NOT NULL
            )"#,
            r#"CREATE TABLE IF NOT EXISTS surface_fingerprints (
                id TEXT PRIMARY KEY,
                program_id TEXT NOT NULL,
                asset_id TEXT NOT NULL,
                fingerprint_type TEXT NOT NULL,
                fingerprint_key TEXT,
                fingerprint_value TEXT NOT NULL,
                rule_id TEXT,
                rule_word TEXT,
                rule_name TEXT,
                normalized_product TEXT,
                normalized_vendor TEXT,
                normalized_category TEXT,
                normalized_family TEXT,
                version TEXT,
                is_primary BOOLEAN DEFAULT FALSE,
                match_source_part TEXT,
                confidence_score DOUBLE PRECISION,
                source TEXT,
                observed_at TIMESTAMPTZ NOT NULL,
                metadata_json TEXT
            )"#,
            r#"CREATE TABLE IF NOT EXISTS surface_asset_classifications (
                id TEXT PRIMARY KEY,
                program_id TEXT NOT NULL,
                asset_id TEXT NOT NULL,
                primary_category TEXT NOT NULL,
                primary_product TEXT NOT NULL,
                primary_vendor TEXT,
                primary_family TEXT,
                rule_id TEXT,
                rule_name TEXT,
                confidence_score DOUBLE PRECISION,
                source TEXT,
                classified_at TIMESTAMPTZ NOT NULL,
                metadata_json TEXT
            )"#,
            r#"CREATE TABLE IF NOT EXISTS surface_evidence (
                id TEXT PRIMARY KEY,
                program_id TEXT NOT NULL,
                asset_id TEXT,
                evidence_type TEXT NOT NULL,
                title TEXT,
                content_text TEXT,
                content_path TEXT,
                content_json TEXT,
                collected_at TIMESTAMPTZ NOT NULL,
                collected_by TEXT,
                probe_node TEXT,
                metadata_json TEXT
            )"#,
            r#"CREATE TABLE IF NOT EXISTS surface_change_logs (
                id TEXT PRIMARY KEY,
                program_id TEXT NOT NULL,
                asset_id TEXT,
                relation_id TEXT,
                change_type TEXT NOT NULL,
                old_value_json TEXT,
                new_value_json TEXT,
                summary TEXT NOT NULL,
                detected_at TIMESTAMPTZ NOT NULL,
                source_run_id TEXT,
                risk_delta DOUBLE PRECISION,
                metadata_json TEXT
            )"#,
            r#"CREATE TABLE IF NOT EXISTS surface_discovery_runs (
                id TEXT PRIMARY KEY,
                program_id TEXT NOT NULL,
                trigger_source TEXT NOT NULL,
                schedule_id TEXT,
                workflow_id TEXT,
                workflow_template_id TEXT,
                plugin_id TEXT,
                status TEXT NOT NULL,
                seed_count INTEGER DEFAULT 0,
                observation_count INTEGER DEFAULT 0,
                imported_asset_count INTEGER DEFAULT 0,
                changed_asset_count INTEGER DEFAULT 0,
                error_message TEXT,
                started_at TIMESTAMPTZ NOT NULL,
                completed_at TIMESTAMPTZ,
                metadata_json TEXT
            )"#,
            r#"CREATE TABLE IF NOT EXISTS surface_observations (
                id TEXT PRIMARY KEY,
                run_id TEXT NOT NULL,
                program_id TEXT NOT NULL,
                artifact_type TEXT NOT NULL,
                object_key TEXT,
                payload_json TEXT NOT NULL,
                source_plugin TEXT,
                confidence_score DOUBLE PRECISION,
                observed_at TIMESTAMPTZ NOT NULL,
                normalized BOOLEAN DEFAULT FALSE,
                metadata_json TEXT
            )"#,
            r#"CREATE TABLE IF NOT EXISTS surface_seeds (
                id TEXT PRIMARY KEY,
                program_id TEXT NOT NULL,
                seed_type TEXT NOT NULL,
                seed_value TEXT NOT NULL,
                status TEXT NOT NULL,
                source TEXT,
                confidence_score DOUBLE PRECISION,
                last_run_at TIMESTAMPTZ,
                metadata_json TEXT,
                created_at TIMESTAMPTZ NOT NULL,
                updated_at TIMESTAMPTZ NOT NULL
            )"#,
            r#"CREATE TABLE IF NOT EXISTS surface_seed_candidates (
                id TEXT PRIMARY KEY,
                program_id TEXT NOT NULL,
                seed_type TEXT NOT NULL,
                seed_value TEXT NOT NULL,
                status TEXT NOT NULL,
                source_asset_id TEXT NOT NULL,
                source_asset_type TEXT NOT NULL,
                source_detail_key TEXT NOT NULL,
                source_display_value TEXT NOT NULL,
                source_canonical_url TEXT,
                confidence_score DOUBLE PRECISION,
                observed_at TIMESTAMPTZ NOT NULL,
                reviewed_at TIMESTAMPTZ,
                metadata_json TEXT,
                created_at TIMESTAMPTZ NOT NULL,
                updated_at TIMESTAMPTZ NOT NULL
            )"#,
        ];

        for sql in table_sql {
            sqlx::query(sql).execute(pool).await?;
        }

        let index_sql = [
            "CREATE INDEX IF NOT EXISTS idx_surface_assets_program ON surface_assets(program_id)",
            "CREATE INDEX IF NOT EXISTS idx_surface_assets_type ON surface_assets(asset_type)",
            "CREATE INDEX IF NOT EXISTS idx_surface_assets_status ON surface_assets(status)",
            "CREATE INDEX IF NOT EXISTS idx_surface_assets_last_seen ON surface_assets(last_seen_at DESC)",
            "CREATE INDEX IF NOT EXISTS idx_surface_assets_discovery_task ON surface_assets(discovery_task_id)",
            "CREATE INDEX IF NOT EXISTS idx_surface_assets_risk ON surface_assets(risk_score DESC)",
            "CREATE INDEX IF NOT EXISTS idx_surface_assets_viewed ON surface_assets(viewed_at)",
            "CREATE INDEX IF NOT EXISTS idx_surface_assets_program_viewed_type ON surface_assets(program_id, viewed_at, asset_type)",
            "CREATE INDEX IF NOT EXISTS idx_surface_assets_program_type_status_seen ON surface_assets(program_id, asset_type, status, last_seen_at DESC, id)",
            "CREATE INDEX IF NOT EXISTS idx_surface_domain_assets_fqdn ON surface_domain_assets(fqdn)",
            "CREATE INDEX IF NOT EXISTS idx_surface_domain_assets_root ON surface_domain_assets(root_domain)",
            "CREATE INDEX IF NOT EXISTS idx_surface_ip_assets_ip ON surface_ip_assets(ip_address)",
            "CREATE INDEX IF NOT EXISTS idx_surface_ip_assets_cidr ON surface_ip_assets(cidr)",
            "CREATE INDEX IF NOT EXISTS idx_surface_host_assets_hostname ON surface_host_assets(hostname)",
            "CREATE INDEX IF NOT EXISTS idx_surface_port_assets_host_port ON surface_port_assets(host_asset_id, port_number, transport_protocol)",
            "CREATE INDEX IF NOT EXISTS idx_surface_service_assets_host_port ON surface_service_assets(host_asset_id, port_number, transport_protocol)",
            "CREATE INDEX IF NOT EXISTS idx_surface_web_assets_url ON surface_web_assets(canonical_url)",
            "CREATE INDEX IF NOT EXISTS idx_surface_cert_assets_sha256 ON surface_cert_assets(sha256)",
            "CREATE INDEX IF NOT EXISTS idx_surface_relations_program ON surface_relations(program_id)",
            "CREATE INDEX IF NOT EXISTS idx_surface_relations_from_asset ON surface_relations(from_asset_id)",
            "CREATE INDEX IF NOT EXISTS idx_surface_relations_to_asset ON surface_relations(to_asset_id)",
            "CREATE INDEX IF NOT EXISTS idx_surface_relations_from_to ON surface_relations(from_asset_id, to_asset_id)",
            "CREATE INDEX IF NOT EXISTS idx_surface_relations_program_from_to_type ON surface_relations(program_id, from_asset_id, to_asset_id, relation_type)",
            "CREATE INDEX IF NOT EXISTS idx_surface_relations_type ON surface_relations(relation_type)",
            "CREATE INDEX IF NOT EXISTS idx_surface_fingerprints_asset ON surface_fingerprints(asset_id)",
            "CREATE INDEX IF NOT EXISTS idx_surface_fingerprints_program_category ON surface_fingerprints(program_id, normalized_category)",
            "CREATE INDEX IF NOT EXISTS idx_surface_fingerprints_program_product ON surface_fingerprints(program_id, normalized_product)",
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_surface_asset_classifications_asset ON surface_asset_classifications(asset_id)",
            "CREATE INDEX IF NOT EXISTS idx_surface_asset_classifications_program_category ON surface_asset_classifications(program_id, primary_category)",
            "CREATE INDEX IF NOT EXISTS idx_surface_asset_classifications_program_product ON surface_asset_classifications(program_id, primary_product)",
            "CREATE INDEX IF NOT EXISTS idx_surface_evidence_asset ON surface_evidence(asset_id)",
            "CREATE INDEX IF NOT EXISTS idx_surface_change_logs_asset ON surface_change_logs(asset_id)",
            "CREATE INDEX IF NOT EXISTS idx_surface_change_logs_program ON surface_change_logs(program_id)",
            "CREATE INDEX IF NOT EXISTS idx_surface_change_logs_source_run ON surface_change_logs(source_run_id)",
            "CREATE INDEX IF NOT EXISTS idx_surface_change_logs_detected ON surface_change_logs(detected_at DESC)",
            "CREATE INDEX IF NOT EXISTS idx_surface_discovery_runs_program ON surface_discovery_runs(program_id)",
            "CREATE INDEX IF NOT EXISTS idx_surface_discovery_runs_started ON surface_discovery_runs(started_at DESC)",
            "CREATE INDEX IF NOT EXISTS idx_surface_observations_run ON surface_observations(run_id)",
            "CREATE INDEX IF NOT EXISTS idx_surface_observations_type ON surface_observations(artifact_type)",
            "CREATE INDEX IF NOT EXISTS idx_surface_observations_plugin_artifact_target ON surface_observations(source_plugin, artifact_type, program_id, object_key, observed_at DESC)",
            "CREATE INDEX IF NOT EXISTS idx_surface_seeds_program ON surface_seeds(program_id)",
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_surface_seed_candidates_identity ON surface_seed_candidates(program_id, seed_type, seed_value, source_asset_id, source_detail_key)",
            "CREATE INDEX IF NOT EXISTS idx_surface_seed_candidates_program_status ON surface_seed_candidates(program_id, status, observed_at DESC)",
        ];

        let surface_asset_alter_columns = [("viewed_at", "TIMESTAMPTZ"), ("viewed_by", "TEXT")];

        for (column, column_type) in surface_asset_alter_columns {
            Self::add_column_if_not_exists_postgres(pool, "surface_assets", column, column_type)
                .await?;
        }

        let alter_columns = [
            ("rule_id", "TEXT"),
            ("rule_word", "TEXT"),
            ("rule_name", "TEXT"),
            ("normalized_product", "TEXT"),
            ("normalized_vendor", "TEXT"),
            ("normalized_category", "TEXT"),
            ("normalized_family", "TEXT"),
            ("version", "TEXT"),
            ("is_primary", "BOOLEAN DEFAULT FALSE"),
            ("match_source_part", "TEXT"),
        ];

        for (column, column_type) in alter_columns {
            Self::add_column_if_not_exists_postgres(
                pool,
                "surface_fingerprints",
                column,
                column_type,
            )
            .await?;
        }

        for sql in index_sql {
            sqlx::query(sql).execute(pool).await?;
        }

        info!("Surface graph migration completed successfully");
        Ok(())
    }

    async fn add_column_if_not_exists_runtime(
        pool: &DatabasePool,
        table: &str,
        column: &str,
        column_type: &str,
    ) -> Result<()> {
        let exists = match pool {
            DatabasePool::PostgreSQL(pg) => {
                sqlx::query_scalar::<_, bool>(
                    "SELECT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = $1 AND column_name = $2)",
                )
                .bind(table)
                .bind(column)
                .fetch_one(pg)
                .await?
            }
            DatabasePool::SQLite(sqlite) => {
                let pragma = format!("PRAGMA table_info({table})");
                let rows = sqlx::query(&pragma).fetch_all(sqlite).await?;
                rows.iter().any(|row| row.get::<String, _>("name") == column)
            }
            DatabasePool::MySQL(mysql) => {
                sqlx::query_scalar::<_, i64>(
                    "SELECT COUNT(*) FROM information_schema.columns WHERE table_schema = DATABASE() AND table_name = ? AND column_name = ?",
                )
                .bind(table)
                .bind(column)
                .fetch_one(mysql)
                .await?
                    > 0
            }
        };

        if !exists {
            let alter_sql = format!("ALTER TABLE {table} ADD COLUMN {column} {column_type}");
            match pool {
                DatabasePool::PostgreSQL(pg) => {
                    sqlx::query(&alter_sql).execute(pg).await?;
                }
                DatabasePool::SQLite(sqlite) => {
                    sqlx::query(&alter_sql).execute(sqlite).await?;
                }
                DatabasePool::MySQL(mysql) => {
                    sqlx::query(&alter_sql).execute(mysql).await?;
                }
            }
        }

        Ok(())
    }

    async fn add_column_if_not_exists_postgres(
        pool: &PgPool,
        table: &str,
        column: &str,
        column_type: &str,
    ) -> Result<()> {
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = $1 AND column_name = $2)",
        )
        .bind(table)
        .bind(column)
        .fetch_one(pool)
        .await?;

        if !exists {
            let alter_sql = format!("ALTER TABLE {table} ADD COLUMN {column} {column_type}");
            sqlx::query(&alter_sql).execute(pool).await?;
        }

        Ok(())
    }
}
