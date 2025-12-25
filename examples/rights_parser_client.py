import requests
import time
from typing import Optional, Dict, Any
from pathlib import Path


class RightsParserClient:
    """Client for Rights Parser API"""
    
    def __init__(self, api_url: str, api_key: str, timeout: int = 300):
        """
        Initialize client
        
        Args:
            api_url: Base URL of API (e.g., https://your-server.com)
            api_key: Your API key
            timeout: Maximum time to wait for processing (seconds)
        """
        self.api_url = api_url.rstrip('/')
        self.api_key = api_key
        self.timeout = timeout
        self.session = requests.Session()
        self.session.headers.update({'X-API-Key': api_key})
    
    def upload_pdf(
        self, 
        file_path: str, 
        webhook_url: Optional[str] = None,
        wait_for_result: bool = True
    ) -> Dict[str, Any]:
        """
        Upload PDF for processing
        
        Args:
            file_path: Path to PDF file
            webhook_url: Optional webhook URL for completion notification
            wait_for_result: If True, wait for processing to complete
        
        Returns:
            Dict containing job info and result (if wait_for_result=True)
        """
        # Upload file
        with open(file_path, 'rb') as f:
            params = {}
            if webhook_url:
                params['webhook_url'] = webhook_url
            
            response = self.session.post(
                f"{self.api_url}/api/v1/upload",
                files={'file': f},
                params=params
            )
            response.raise_for_status()
        
        job = response.json()
        job_id = job['job_id']
        
        if not wait_for_result:
            return job
        
        # Wait for processing
        result = self.wait_for_completion(job_id)
        return result
    
    def wait_for_completion(
        self, 
        job_id: str, 
        poll_interval: int = 10
    ) -> Dict[str, Any]:
        """
        Wait for job to complete
        
        Args:
            job_id: Job ID to wait for
            poll_interval: Seconds between status checks
        
        Returns:
            Final result
        """
        start_time = time.time()
        
        while True:
            # Check timeout
            if time.time() - start_time > self.timeout:
                raise TimeoutError(f"Job {job_id} did not complete within {self.timeout}s")
            
            # Get status
            status = self.get_status(job_id)
            
            if status['status'] == 'completed':
                return self.get_result(job_id)
            
            elif status['status'] == 'failed':
                error_msg = status.get('error_message', 'Unknown error')
                raise Exception(f"Job failed: {error_msg}")
            
            # Still processing, wait
            time.sleep(poll_interval)
    
    def get_status(self, job_id: str) -> Dict[str, Any]:
        """
        Get job status
        
        Args:
            job_id: Job ID
        
        Returns:
            Status information
        """
        response = self.session.get(f"{self.api_url}/api/v1/status/{job_id}")
        response.raise_for_status()
        return response.json()
    
    def get_result(self, job_id: str) -> Dict[str, Any]:
        """
        Get job result
        
        Args:
            job_id: Job ID
        
        Returns:
            Result including parsed data
        """
        response = self.session.get(f"{self.api_url}/api/v1/result/{job_id}")
        response.raise_for_status()
        return response.json()
    
    def list_jobs(self) -> list:
        """
        List recent jobs
        
        Returns:
            List of recent jobs
        """
        response = self.session.get(f"{self.api_url}/api/v1/jobs")
        response.raise_for_status()
        return response.json()
    
    def health(self) -> Dict[str, Any]:
        """
        Check API health
        
        Returns:
            Health status
        """
        response = self.session.get(f"{self.api_url}/health")
        response.raise_for_status()
        return response.json()


# Example usage
if __name__ == "__main__":
    # Initialize client
    client = RightsParserClient(
        api_url="https://your-server.com",
        api_key="your_api_key_here"
    )
    
    # Check health
    health = client.health()
    print(f"API Status: {health['status']}")
    
    # Upload and process PDF (wait for result)
    print("Uploading PDF...")
    result = client.upload_pdf(
        file_path="./Kalki.pdf",
        wait_for_result=True
    )
    
    print(f"✅ Processing completed!")
    print(f"Job ID: {result['job_id']}")
    print(f"IPFS CID: {result['ipfs_cid']}")
    print(f"Processing Time: {result['metadata']['processing_time_ms']}ms")
    print(f"\nParsed Data:")
    print(result['parsed_data'])
    
    # Or upload without waiting
    job = client.upload_pdf(
        file_path="./document.pdf",
        wait_for_result=False
    )
    
    print(f"Job created: {job['job_id']}")
    print("Check status later with client.get_status(job_id)")