"""
sp-service E2E Tests - Deep Research Tests
Tests the deep research endpoint with agentic workflow.
"""
import pytest
import requests
import json
import time

REQUEST_TIMEOUT = 30
STREAM_TIMEOUT = 600  # Deep research can take longer


class TestDeepResearch:
    """Tests for /v1/research/deep endpoint."""

    def test_deep_research_basic(self, base_url, session, deep_research_query):
        """Test basic deep research request."""
        payload = {
            "query": deep_research_query,
            "agentic_mode": True,
            "max_iterations": 3
        }
        
        response = session.post(
            f"{base_url}/v1/research/deep",
            json=payload,
            stream=True,
            timeout=STREAM_TIMEOUT
        )
        
        assert response.status_code == 200
        
        # Parse SSE stream
        events = []
        for line in response.iter_lines():
            if line:
                line = line.decode('utf-8')
                if line.startswith('data: '):
                    try:
                        data = json.loads(line[6:])
                        events.append(data)
                    except json.JSONDecodeError:
                        pass
        
        # Should have received events
        assert len(events) > 0

    def test_deep_research_stages(self, base_url, session, deep_research_query):
        """Test deep research workflow stages."""
        payload = {
            "query": deep_research_query,
            "agentic_mode": True,
            "max_iterations": 3
        }
        
        response = session.post(
            f"{base_url}/v1/research/deep",
            json=payload,
            stream=True,
            timeout=STREAM_TIMEOUT
        )
        
        # Parse stages from stream
        stages = []
        for line in response.iter_lines():
            if line:
                line = line.decode('utf-8')
                if line.startswith('data: '):
                    try:
                        data = json.loads(line[6:])
                        if 'stage' in data:
                            stages.append(data['stage'])
                    except json.JSONDecodeError:
                        pass
        
        # Should have planning stage at minimum
        assert len(stages) > 0

    def test_deep_research_macro_economy(self, base_url, session, macro_query):
        """Test deep research for macroeconomic analysis."""
        payload = {
            "query": macro_query,
            "agentic_mode": True,
            "max_iterations": 3
        }
        
        response = session.post(
            f"{base_url}/v1/research/deep",
            json=payload,
            stream=True,
            timeout=STREAM_TIMEOUT
        )
        
        assert response.status_code == 200
        
        # Collect full response
        full_response = ""
        for line in response.iter_lines():
            if line:
                full_response += line.decode('utf-8') + "\n"
        
        # Should have some content
        assert len(full_response) > 0

    def test_deep_research_without_agentic(self, base_url, session):
        """Test deep research without agentic mode."""
        payload = {
            "query": "Ola, responda brevemente",
            "agentic_mode": False
        }
        
        response = session.post(
            f"{base_url}/v1/research/deep",
            json=payload,
            stream=True,
            timeout=STREAM_TIMEOUT
        )
        
        assert response.status_code == 200

    def test_deep_research_empty_query(self, base_url, session):
        """Test deep research with empty query."""
        payload = {
            "query": "",
            "agentic_mode": True
        }
        
        response = session.post(
            f"{base_url}/v1/research/deep",
            json=payload,
            stream=True,
            timeout=REQUEST_TIMEOUT
        )
        
        # Should handle gracefully (error or empty result)
        assert response.status_code in [200, 400]


class TestDeepResearchTools:
    """Tests for deep research tool calling."""

    def test_deep_research_financial(self, base_url, session):
        """Test deep research with financial data."""
        payload = {
            "query": "Analise as acoes da PETR4 e VALE3 dos ultimos 6 meses",
            "agentic_mode": True,
            "max_iterations": 3
        }
        
        response = session.post(
            f"{base_url}/v1/research/deep",
            json=payload,
            stream=True,
            timeout=STREAM_TIMEOUT
        )
        
        assert response.status_code == 200

    def test_deep_research_with_tools(self, base_url, session):
        """Test deep research explicitly requesting tools."""
        payload = {
            "query": "Busque dados do IPCA e SELIC usando as ferramentas disponiveis",
            "agentic_mode": True,
            "tools": ["financial", "macro"],
            "max_iterations": 3
        }
        
        response = session.post(
            f"{base_url}/v1/research/deep",
            json=payload,
            stream=True,
            timeout=STREAM_TIMEOUT
        )
        
        assert response.status_code == 200


class TestDeepResearchValidation:
    """Tests for deep research input validation."""

    def test_deep_research_missing_query(self, base_url, session):
        """Test deep research without query field."""
        payload = {
            "agentic_mode": True
        }
        
        response = session.post(
            f"{base_url}/v1/research/deep",
            json=payload,
            timeout=REQUEST_TIMEOUT
        )
        
        # Should return error
        assert response.status_code in [400, 422]

    def test_deep_research_invalid_iterations(self, base_url, session):
        """Test deep research with invalid max_iterations."""
        payload = {
            "query": "Test",
            "agentic_mode": True,
            "max_iterations": -1
        }
        
        response = session.post(
            f"{base_url}/v1/research/deep",
            json=payload,
            timeout=REQUEST_TIMEOUT
        )
        
        # Should handle gracefully
        assert response.status_code in [200, 400]
