-- PostgreSQL Schema for Rights Parser API

-- Jobs table - tracks all PDF processing jobs
CREATE TABLE jobs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    
    -- File information
    file_name VARCHAR(255) NOT NULL,
    file_path VARCHAR(500) NOT NULL,
    file_size BIGINT NOT NULL,
    
    -- API authentication
    api_key_hash VARCHAR(64) NOT NULL,
    user_id VARCHAR(100),
    
    -- Processing status
    status VARCHAR(20) NOT NULL DEFAULT 'pending',
    -- Status values: pending, processing, completed, failed
    
    -- Results
    ipfs_cid VARCHAR(100),
    encryption_key TEXT,
    parsed_json JSONB,
    
    -- Error handling
    error_message TEXT,
    retry_count INTEGER DEFAULT 0,
    
    -- Timestamps
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    started_at TIMESTAMP WITH TIME ZONE,
    completed_at TIMESTAMP WITH TIME ZONE,
    
    -- Metadata
    processing_time_ms BIGINT,
    model_used VARCHAR(50) DEFAULT 'llama3.3:70b-instruct-q4_K_M',
    
    -- Webhook
    webhook_url TEXT,
    webhook_sent BOOLEAN DEFAULT FALSE,
    
    -- Indexing
    CONSTRAINT status_check CHECK (status IN ('pending', 'processing', 'completed', 'failed'))
);

-- Indexes for performance
CREATE INDEX idx_jobs_status ON jobs(status);
CREATE INDEX idx_jobs_created_at ON jobs(created_at DESC);
CREATE INDEX idx_jobs_api_key_hash ON jobs(api_key_hash);
CREATE INDEX idx_jobs_completed_at ON jobs(completed_at DESC) WHERE status = 'completed';

-- API Keys table - manage multiple API keys
CREATE TABLE api_keys (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    
    -- Key information
    key_hash VARCHAR(64) NOT NULL UNIQUE,
    key_prefix VARCHAR(10) NOT NULL, -- First 8 chars for identification
    name VARCHAR(100),
    
    -- Ownership
    user_id VARCHAR(100),
    organization VARCHAR(100),
    
    -- Permissions
    is_active BOOLEAN DEFAULT TRUE,
    rate_limit INTEGER DEFAULT 100, -- requests per hour
    
    -- Usage tracking
    requests_count BIGINT DEFAULT 0,
    last_used_at TIMESTAMP WITH TIME ZONE,
    
    -- Timestamps
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    expires_at TIMESTAMP WITH TIME ZONE,
    
    -- Metadata
    created_by VARCHAR(100),
    notes TEXT
);

-- Index for fast API key lookup
CREATE INDEX idx_api_keys_hash ON api_keys(key_hash);
CREATE INDEX idx_api_keys_active ON api_keys(is_active) WHERE is_active = TRUE;

-- Usage logs for analytics
CREATE TABLE usage_logs (
    id BIGSERIAL PRIMARY KEY,
    
    job_id UUID REFERENCES jobs(id),
    api_key_hash VARCHAR(64),
    
    endpoint VARCHAR(100),
    method VARCHAR(10),
    status_code INTEGER,
    
    processing_time_ms BIGINT,
    file_size BIGINT,
    
    ip_address INET,
    user_agent TEXT,
    
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Index for analytics queries
CREATE INDEX idx_usage_logs_created_at ON usage_logs(created_at DESC);
CREATE INDEX idx_usage_logs_api_key ON usage_logs(api_key_hash);

-- Function to cleanup old jobs (optional)
CREATE OR REPLACE FUNCTION cleanup_old_jobs()
RETURNS void AS $$
BEGIN
    -- Delete jobs older than 30 days
    DELETE FROM jobs 
    WHERE created_at < NOW() - INTERVAL '30 days'
    AND status IN ('completed', 'failed');
END;
$$ LANGUAGE plpgsql;

-- Example: Schedule cleanup (requires pg_cron extension)
-- SELECT cron.schedule('cleanup-old-jobs', '0 2 * * *', 'SELECT cleanup_old_jobs()');

-- Sample API key insertion
-- INSERT INTO api_keys (key_hash, key_prefix, name, user_id)
-- VALUES (
--     encode(sha256('your-api-key'::bytea), 'hex'),
--     'sk_test_',
--     'Test API Key',
--     'user-123'
-- );