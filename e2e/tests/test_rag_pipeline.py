"""
sp-service E2E Tests - RAG Pipeline Tests
Tests the RAG ingestion, search, and chat endpoints.
"""
import pytest
import requests
import json
import time
import tempfile
import os

REQUEST_TIMEOUT = 30
STREAM_TIMEOUT = 300


class TestRAGIngestion:
    """Tests for /v1/rag/ingest endpoint."""

    def test_rag_ingest_markdown(self, base_url, session, test_document_content, workspace_id):
        """Test RAG ingestion of markdown content."""
        # Create temporary markdown file
        with tempfile.NamedTemporaryFile(mode='w', suffix='.md', delete=False) as f:
            f.write(test_document_content)
            temp_file = f.name
        
        try:
            # Upload file
            with open(temp_file, 'rb') as f:
                files = {'file': f}
                data = {'project_id': workspace_id}
                
                response = session.post(
                    f"{base_url}/v1/rag/ingest",
                    files=files,
                    data=data,
                    timeout=REQUEST_TIMEOUT
                )
            
            assert response.status_code == 200
            result = response.json()
            
            # Validate response
            assert "id" in result or "status" in result
        finally:
            # Cleanup
            os.unlink(temp_file)

    def test_rag_ingest_invalid_file(self, base_url, session, workspace_id):
        """Test RAG ingestion with invalid file type."""
        # Create temporary binary file
        with tempfile.NamedTemporaryFile(mode='wb', suffix='.xyz', delete=False) as f:
            f.write(b'invalid content')
            temp_file = f.name
        
        try:
            with open(temp_file, 'rb') as f:
                files = {'file': f}
                data = {'project_id': workspace_id}
                
                response = session.post(
                    f"{base_url}/v1/rag/ingest",
                    files=files,
                    data=data,
                    timeout=REQUEST_TIMEOUT
                )
            
            # Should handle or reject unsupported format
            assert response.status_code in [200, 400, 415]
        finally:
            os.unlink(temp_file)


class TestRAGSearch:
    """Tests for /v1/rag/search endpoint."""

    def test_rag_search_basic(self, base_url, session):
        """Test basic RAG search."""
        params = {
            "q": "inflacao IPCA",
            "limit": 5
        }
        
        response = session.get(
            f"{base_url}/v1/rag/search",
            params=params,
            timeout=REQUEST_TIMEOUT
        )
        
        # Should return results or empty list
        assert response.status_code == 200
        data = response.json()
        
        # Validate response structure
        assert isinstance(data, dict)
        if "results" in data:
            assert isinstance(data["results"], list)

    def test_rag_search_limit(self, base_url, session):
        """Test RAG search with different limits."""
        for limit in [1, 5, 10]:
            params = {
                "q": "test",
                "limit": limit
            }
            
            response = session.get(
                f"{base_url}/v1/rag/search",
                params=params,
                timeout=REQUEST_TIMEOUT
            )
            
            assert response.status_code == 200

    def test_rag_search_empty_query(self, base_url, session):
        """Test RAG search with empty query."""
        params = {
            "q": "",
            "limit": 5
        }
        
        response = session.get(
            f"{base_url}/v1/rag/search",
            params=params,
            timeout=REQUEST_TIMEOUT
        )
        
        # Should handle gracefully
        assert response.status_code in [200, 400]


class TestRAGChat:
    """Tests for /v1/rag/chat endpoint."""

    def test_rag_chat_basic(self, base_url, session):
        """Test basic RAG chat."""
        payload = {
            "query": "Qual e a inflacao acumulada?",
            "use_rag": True
        }
        
        response = session.post(
            f"{base_url}/v1/rag/chat",
            json=payload,
            timeout=STREAM_TIMEOUT
        )
        
        assert response.status_code == 200
        data = response.json()
        
        # Should have answer or sources
        assert "answer" in data or "response" in data or "sources" in data

    def test_rag_chat_without_rag(self, base_url, session):
        """Test chat without RAG enabled."""
        payload = {
            "query": "Ola, tudo bem?",
            "use_rag": False
        }
        
        response = session.post(
            f"{base_url}/v1/rag/chat",
            json=payload,
            timeout=STREAM_TIMEOUT
        )
        
        assert response.status_code == 200

    def test_rag_chat_with_top_k(self, base_url, session):
        """Test RAG chat with custom top_k parameter."""
        payload = {
            "query": "Qual e a inflacao?",
            "use_rag": True,
            "top_k": 10
        }
        
        response = session.post(
            f"{base_url}/v1/rag/chat",
            json=payload,
            timeout=STREAM_TIMEOUT
        )
        
        assert response.status_code == 200


class TestRAGPipeline:
    """Integration tests for complete RAG pipeline."""

    def test_rag_ingest_search_flow(self, base_url, session, test_document_content, workspace_id):
        """Test complete RAG flow: ingest -> search."""
        # Step 1: Ingest document
        with tempfile.NamedTemporaryFile(mode='w', suffix='.md', delete=False) as f:
            f.write(test_document_content)
            temp_file = f.name
        
        try:
            with open(temp_file, 'rb') as f:
                files = {'file': f}
                data = {'project_id': workspace_id}
                
                ingest_response = session.post(
                    f"{base_url}/v1/rag/ingest",
                    files=files,
                    data=data,
                    timeout=REQUEST_TIMEOUT
                )
            
            assert ingest_response.status_code == 200
            
            # Wait for indexing
            time.sleep(2)
            
            # Step 2: Search for content
            search_response = session.get(
                f"{base_url}/v1/rag/search",
                params={"q": "IPCA inflacao", "limit": 5},
                timeout=REQUEST_TIMEOUT
            )
            
            assert search_response.status_code == 200
            
        finally:
            os.unlink(temp_file)
