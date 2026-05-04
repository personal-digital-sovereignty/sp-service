"""
sp-service E2E Tests - Health Check Tests
Tests the basic health and readiness endpoints.
"""
import pytest
import requests


class TestHealthEndpoints:
    """Tests for health check endpoints."""

    def test_health_endpoint(self, base_url, session):
        """Test /health endpoint returns healthy status."""
        response = session.get(f"{base_url}/health", timeout=REQUEST_TIMEOUT)
        
        assert response.status_code == 200
        data = response.json()
        assert data.get("status") == "healthy"
        assert "version" in data
        assert "timestamp" in data

    def test_ready_endpoint(self, base_url, session):
        """Test /ready endpoint returns readiness status."""
        response = session.get(f"{base_url}/ready", timeout=REQUEST_TIMEOUT)
        
        assert response.status_code == 200
        data = response.json()
        assert "ready" in data
        assert "checks" in data

    def test_health_endpoint_format(self, base_url, session):
        """Test /health endpoint returns valid JSON format."""
        response = session.get(f"{base_url}/health", timeout=REQUEST_TIMEOUT)
        
        # Validate response is JSON
        assert "application/json" in response.headers.get("Content-Type", "")
        
        # Validate structure
        data = response.json()
        assert isinstance(data, dict)
        assert "status" in data
        assert isinstance(data["status"], str)

    def test_service_version(self, base_url, session):
        """Test service version is present in health response."""
        response = session.get(f"{base_url}/health", timeout=REQUEST_TIMEOUT)
        data = response.json()
        
        version = data.get("version", "")
        assert version != ""
        assert version.startswith("1.") or version.startswith("v")


class TestTelemetryEndpoints:
    """Tests for telemetry endpoints."""

    def test_telemetry_endpoint(self, base_url, session):
        """Test /v1/telemetry endpoint returns hardware metrics."""
        response = session.get(f"{base_url}/v1/telemetry", timeout=REQUEST_TIMEOUT)
        
        assert response.status_code == 200
        data = response.json()
        
        # Should have RAM info at minimum
        assert "ram" in data or "memory" in data

    def test_telemetry_format(self, base_url, session):
        """Test /v1/telemetry returns valid JSON format."""
        response = session.get(f"{base_url}/v1/telemetry", timeout=REQUEST_TIMEOUT)
        
        assert "application/json" in response.headers.get("Content-Type", "")
        data = response.json()
        assert isinstance(data, dict)


class TestServiceAvailability:
    """Tests for service availability."""

    def test_service_startup(self, health_check):
        """Test service starts up and passes health check."""
        # This test depends on the health_check fixture
        # If fixture passes, service is available
        assert health_check.get("status") == "healthy"

    def test_concurrent_health_checks(self, base_url, session):
        """Test multiple concurrent health checks."""
        import concurrent.futures
        
        def check_health():
            try:
                response = session.get(f"{base_url}/health", timeout=REQUEST_TIMEOUT)
                return response.status_code == 200
            except:
                return False
        
        # Run 10 concurrent health checks
        with concurrent.futures.ThreadPoolExecutor(max_workers=10) as executor:
            futures = [executor.submit(check_health) for _ in range(10)]
            results = [f.result() for f in futures]
        
        # All should succeed
        assert all(results), "Some concurrent health checks failed"
