"""
sp-service E2E Tests - Shared Fixtures
"""
import pytest
import requests
import time
import os

# Configuration
BASE_URL = os.environ.get("SP_SERVICE_URL", "http://localhost:8080")
REQUEST_TIMEOUT = 30
STREAM_TIMEOUT = 300


@pytest.fixture(scope="session")
def base_url():
    """Returns the base URL for the sp-service API."""
    return BASE_URL


@pytest.fixture(scope="session")
def session():
    """Creates a requests session with default configuration."""
    session = requests.Session()
    session.headers.update({
        "Content-Type": "application/json",
        "Accept": "application/json"
    })
    return session


@pytest.fixture(scope="session")
def health_check(session):
    """Performs initial health check and waits for service to be ready."""
    max_retries = 10
    retry_delay = 2
    
    for i in range(max_retries):
        try:
            response = session.get(f"{BASE_URL}/health", timeout=REQUEST_TIMEOUT)
            if response.status_code == 200:
                data = response.json()
                assert data.get("status") == "healthy"
                return data
        except requests.exceptions.ConnectionError:
            if i < max_retries - 1:
                time.sleep(retry_delay)
            else:
                pytest.fail(f"sp-service not available at {BASE_URL} after {max_retries} retries")
    
    pytest.fail(f"Health check failed after {max_retries} retries")


@pytest.fixture
def workspace_id():
    """Returns a unique workspace ID for testing."""
    return f"test-workspace-{int(time.time())}"


@pytest.fixture
def test_document_content():
    """Returns sample document content for RAG tests."""
    return """# Relatório de Inflação - IPCA 2024

## Resumo
O IPCA acumulado em 12 meses atingiu 4.5% em dezembro de 2024.

## Meses
- Janeiro 2024: 0.42%
- Fevereiro 2024: 0.83%
- Março 2024: 0.16%
- Abril 2024: 0.38%
- Maio 2024: 0.46%
- Junho 2024: 0.21%
- Julho 2024: 0.27%
- Agosto 2024: 0.30%
- Setembro 2024: 0.39%
- Outubro 2024: 0.42%
- Novembro 2024: 0.38%
- Dezembro 2024: 0.35%

## Acumulado 12 meses: 4.5%
"""


@pytest.fixture
def macro_query():
    """Returns a sample macroeconomic query for testing."""
    return "Analise o IPCA e SELIC dos últimos 12 meses"


@pytest.fixture
def deep_research_query():
    """Returns a sample deep research query for testing."""
    return "Faça uma análise macroeconômica completa do Brasil 2024-2025 incluindo IPCA, SELIC, Dólar e PIB"


def parse_sse_stream(response_text):
    """Parses SSE stream response into list of data objects."""
    events = []
    for line in response_text.split('\n'):
        if line.startswith('data: '):
            try:
                import json
                data = json.loads(line[6:])
                events.append(data)
            except json.JSONDecodeError:
                pass
    return events


def wait_for_service(base_url, max_retries=10, retry_delay=2):
    """Waits for the service to be available."""
    for i in range(max_retries):
        try:
            response = requests.get(f"{base_url}/health", timeout=REQUEST_TIMEOUT)
            if response.status_code == 200:
                return True
        except requests.exceptions.ConnectionError:
            if i < max_retries - 1:
                time.sleep(retry_delay)
    return False
