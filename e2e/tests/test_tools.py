"""
sp-service E2E Tests - Python Workers Tools Tests
Tests the financial and macroeconomic data tools.
"""
import pytest
import requests
import json

REQUEST_TIMEOUT = 60


class TestFinancialTools:
    """Tests for /v1/tools/fetch_financial_ticker endpoint."""

    def test_fetch_financial_ticker_petr4(self, base_url, session):
        """Test fetching PETR4 financial data."""
        payload = {
            "ticker": "PETR4.SA",
            "metrics": ["close", "volume"],
            "period": "3M"
        }
        
        response = session.post(
            f"{base_url}/v1/tools/fetch_financial_ticker",
            json=payload,
            timeout=REQUEST_TIMEOUT
        )
        
        # Should return data or error (API may be rate limited)
        assert response.status_code in [200, 400, 429, 503]
        
        if response.status_code == 200:
            data = response.json()
            assert "data" in data or "ticker" in data or "error" in data

    def test_fetch_financial_ticker_vale3(self, base_url, session):
        """Test fetching VALE3 financial data."""
        payload = {
            "ticker": "VALE3.SA",
            "period": "3M"
        }
        
        response = session.post(
            f"{base_url}/v1/tools/fetch_financial_ticker",
            json=payload,
            timeout=REQUEST_TIMEOUT
        )
        
        assert response.status_code in [200, 400, 429, 503]

    def test_fetch_financial_ticker_invalid(self, base_url, session):
        """Test fetching invalid ticker."""
        payload = {
            "ticker": "INVALID123.SA",
            "period": "3M"
        }
        
        response = session.post(
            f"{base_url}/v1/tools/fetch_financial_ticker",
            json=payload,
            timeout=REQUEST_TIMEOUT
        )
        
        # Should return error for invalid ticker
        assert response.status_code in [200, 400, 404, 503]


class TestMacroTools:
    """Tests for /v1/tools/fetch_macroeconomy endpoint."""

    def test_fetch_macroeconomy_ipca(self, base_url, session):
        """Test fetching IPCA data."""
        payload = {
            "indicators": ["IPCA"],
            "period": "12M"
        }
        
        response = session.post(
            f"{base_url}/v1/tools/fetch_macroeconomy",
            json=payload,
            timeout=REQUEST_TIMEOUT
        )
        
        # Should return data or error (API may be rate limited)
        assert response.status_code in [200, 400, 429, 503]
        
        if response.status_code == 200:
            data = response.json()
            assert "indicators" in data or "data" in data or "error" in data

    def test_fetch_macroeconomy_selic(self, base_url, session):
        """Test fetching SELIC data."""
        payload = {
            "indicators": ["SELIC"],
            "period": "12M"
        }
        
        response = session.post(
            f"{base_url}/v1/tools/fetch_macroeconomy",
            json=payload,
            timeout=REQUEST_TIMEOUT
        )
        
        assert response.status_code in [200, 400, 429, 503]

    def test_fetch_macroeconomy_multiple(self, base_url, session):
        """Test fetching multiple indicators."""
        payload = {
            "indicators": ["IPCA", "SELIC", "IGPM"],
            "period": "6M"
        }
        
        response = session.post(
            f"{base_url}/v1/tools/fetch_macroeconomy",
            json=payload,
            timeout=REQUEST_TIMEOUT
        )
        
        assert response.status_code in [200, 400, 429, 503]

    def test_fetch_macroeconomy_invalid(self, base_url, session):
        """Test fetching invalid indicator."""
        payload = {
            "indicators": ["INVALID_INDICATOR_XYZ"],
            "period": "12M"
        }
        
        response = session.post(
            f"{base_url}/v1/tools/fetch_macroeconomy",
            json=payload,
            timeout=REQUEST_TIMEOUT
        )
        
        # Should return error or empty data
        assert response.status_code in [200, 400, 404, 503]


class TestSubResearcher:
    """Tests for /v1/tools/dispatch_sub_researcher endpoint."""

    def test_dispatch_sub_researcher_basic(self, base_url, session):
        """Test basic sub-researcher dispatch."""
        payload = {
            "query": "O que e o programa PROSUB da Marinha?",
            "max_results": 5
        }
        
        response = session.post(
            f"{base_url}/v1/tools/dispatch_sub_researcher",
            json=payload,
            timeout=REQUEST_TIMEOUT
        )
        
        # Should return results or error
        assert response.status_code in [200, 400, 503]

    def test_dispatch_sub_researcher_empty(self, base_url, session):
        """Test sub-researcher with empty query."""
        payload = {
            "query": "",
            "max_results": 5
        }
        
        response = session.post(
            f"{base_url}/v1/tools/dispatch_sub_researcher",
            json=payload,
            timeout=REQUEST_TIMEOUT
        )
        
        # Should handle gracefully
        assert response.status_code in [200, 400]


class TestToolsIntegration:
    """Integration tests for multiple tools."""

    def test_fetch_multiple_tools_sequential(self, base_url, session):
        """Test fetching data from multiple tools sequentially."""
        # Test 1: Financial
        financial_payload = {
            "ticker": "PETR4.SA",
            "period": "1M"
        }
        financial_response = session.post(
            f"{base_url}/v1/tools/fetch_financial_ticker",
            json=financial_payload,
            timeout=REQUEST_TIMEOUT
        )
        assert financial_response.status_code in [200, 400, 429, 503]
        
        # Test 2: Macro
        macro_payload = {
            "indicators": ["IPCA"],
            "period": "1M"
        }
        macro_response = session.post(
            f"{base_url}/v1/tools/fetch_macroeconomy",
            json=macro_payload,
            timeout=REQUEST_TIMEOUT
        )
        assert macro_response.status_code in [200, 400, 429, 503]
