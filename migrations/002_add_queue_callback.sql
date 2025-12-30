-- Add queue_id and callback_url columns to jobs table
ALTER TABLE jobs ADD COLUMN IF NOT EXISTS queue_id INTEGER;
ALTER TABLE jobs ADD COLUMN IF NOT EXISTS callback_url TEXT;

-- Add index for queue_id lookups
CREATE INDEX IF NOT EXISTS idx_jobs_queue_id ON jobs(queue_id);
