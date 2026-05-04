"""
sp-service E2E Tests - Chat API Tests
Tests the chat completions endpoint with and without streaming.
"""
import pytest
import requests
import json

REQUEST_TIMEOUT = 30
STREAM_TIMEOUT = 300


class TestChatCompletions:
    """Tests for /v1/chat/completions endpoint."""

    def test_chat_basic(self, base_url, session):
        """Test basic chat request without streaming."""
        payload = {
            "model": "qwen3:8b",
            "messages": [
                {"role": "user", "content": "Ola, responda apenas 'OK'"}
            ],
            "stream": False
        }
        
        response = session.post(
            f"{base_url}/v1/chat/completions",
            json=payload,
            timeout=STREAM_TIMEOUT
        )
        
        assert response.status_code == 200
        data = response.json()
        
        # Validate response structure
        assert "choices" in data or "message" in data or "content" in data

    def test_chat_streaming(self, base_url, session):
        """Test chat request with streaming enabled."""
        payload = {
            "model": "qwen3:8b",
            "messages": [
                {"role": "user", "content": "Conte de 1 a 3"}
            ],
            "stream": True
        }
        
        response = session.post(
            f"{base_url}/v1/chat/completions",
            json=payload,
            stream=True,
            timeout=STREAM_TIMEOUT
        )
        
        assert response.status_code == 200
        
        # Parse SSE stream
        chunks = []
        for line in response.iter_lines():
            if line:
                line = line.decode('utf-8')
                if line.startswith('data: '):
                    chunks.append(line[6:])
        
        # Should have received multiple chunks
        assert len(chunks) > 0

    def test_chat_multi_turn(self, base_url, session):
        """Test multi-turn conversation."""
        messages = [
            {"role": "user", "content": "Qual e a capital da Franca?"},
            {"role": "assistant", "content": "A capital da Franca e Paris."},
            {"role": "user", "content": "Qual a populacao de Paris?"}
        ]
        
        payload = {
            "model": "qwen3:8b",
            "messages": messages,
            "stream": False
        }
        
        response = session.post(
            f"{base_url}/v1/chat/completions",
            json=payload,
            timeout=STREAM_TIMEOUT
        )
        
        assert response.status_code == 200
        data = response.json()
        
        # Response should contain answer about Paris population
        response_text = str(data).lower()
        assert "paris" in response_text or "milhao" in response_text or "habitantes" in response_text

    def test_chat_empty_messages(self, base_url, session):
        """Test chat with empty messages array."""
        payload = {
            "model": "qwen3:8b",
            "messages": [],
            "stream": False
        }
        
        response = session.post(
            f"{base_url}/v1/chat/completions",
            json=payload,
            timeout=REQUEST_TIMEOUT
        )
        
        # Should return error or handle gracefully
        assert response.status_code in [200, 400, 422]

    def test_chat_invalid_model(self, base_url, session):
        """Test chat with non-existent model."""
        payload = {
            "model": "nonexistent-model-xyz",
            "messages": [
                {"role": "user", "content": "Test"}
            ],
            "stream": False
        }
        
        response = session.post(
            f"{base_url}/v1/chat/completions",
            json=payload,
            timeout=REQUEST_TIMEOUT
        )
        
        # Should return error (model not found)
        assert response.status_code in [400, 404, 503]


class TestChatToolCalling:
    """Tests for chat with tool calling."""

    def test_chat_with_workspace(self, base_url, session, workspace_id):
        """Test chat with workspace_id parameter."""
        payload = {
            "model": "qwen3:8b",
            "messages": [
                {"role": "user", "content": "Ola"}
            ],
            "workspace_id": workspace_id,
            "stream": False
        }
        
        response = session.post(
            f"{base_url}/v1/chat/completions",
            json=payload,
            timeout=STREAM_TIMEOUT
        )
        
        assert response.status_code == 200

    def test_chat_with_session(self, base_url, session):
        """Test chat with session_id for context persistence."""
        session_id = f"test-session-{int(time.time())}"
        
        # First message
        payload1 = {
            "model": "qwen3:8b",
            "messages": [
                {"role": "user", "content": "Meu nome e João"}
            ],
            "session_id": session_id,
            "stream": False
        }
        
        response1 = session.post(
            f"{base_url}/v1/chat/completions",
            json=payload1,
            timeout=STREAM_TIMEOUT
        )
        
        assert response1.status_code == 200
        
        # Second message (should remember name)
        payload2 = {
            "model": "qwen3:8b",
            "messages": [
                {"role": "user", "content": "Qual meu nome?"}
            ],
            "session_id": session_id,
            "stream": False
        }
        
        response2 = session.post(
            f"{base_url}/v1/chat/completions",
            json=payload2,
            timeout=STREAM_TIMEOUT
        )
        
        assert response2.status_code == 200


class TestChatValidation:
    """Tests for chat input validation."""

    def test_chat_missing_model(self, base_url, session):
        """Test chat request without model field."""
        payload = {
            "messages": [
                {"role": "user", "content": "Test"}
            ]
        }
        
        response = session.post(
            f"{base_url}/v1/chat/completions",
            json=payload,
            timeout=REQUEST_TIMEOUT
        )
        
        # Should return error (missing required field)
        assert response.status_code in [400, 422]

    def test_chat_very_long_message(self, base_url, session):
        """Test chat with very long message."""
        long_text = "A" * 10000  # 10KB message
        
        payload = {
            "model": "qwen3:8b",
            "messages": [
                {"role": "user", "content": long_text}
            ],
            "stream": False
        }
        
        response = session.post(
            f"{base_url}/v1/chat/completions",
            json=payload,
            timeout=STREAM_TIMEOUT
        )
        
        # Should handle or reject gracefully
        assert response.status_code in [200, 400, 413]
