import pytest
import sys
import os
import json

sys.path.append(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

from culture_matrix import fetch_cultural_data

def test_fetch_cultural_data_musicbrainz(monkeypatch, capsys):
    def mock_urlopen(req, *args, **kwargs):
        class MockResponse:
            def read(self):
                return b'{"artists": [{"name": "Test Artist", "type": "Group", "tags": [{"name":"rock"}], "id": "123"}]}'
            def __enter__(self):
                return self
            def __exit__(self, exc_type, exc_val, exc_tb):
                pass
        return MockResponse()
    
    monkeypatch.setattr('urllib.request.urlopen', mock_urlopen)
    
    fetch_cultural_data("Beatles", "MUSICBRAINZ")
    captured = capsys.readouterr()
    result = json.loads(captured.out)
    
    assert result["status"] == "success"
    assert "Test Artist" in result["data_compressed"]

def test_fetch_cultural_data_wikipedia(monkeypatch, capsys):
    def mock_urlopen(req, *args, **kwargs):
        class MockResponse:
            def read(self):
                return b'{"extract": "Resumo da Wikipedia sobre o termo."}'
            def __enter__(self):
                return self
            def __exit__(self, exc_type, exc_val, exc_tb):
                pass
        return MockResponse()
    
    monkeypatch.setattr('urllib.request.urlopen', mock_urlopen)
    
    fetch_cultural_data("Inteligencia_Artificial", "WIKIPEDIA")
    captured = capsys.readouterr()
    result = json.loads(captured.out)
    
    assert result["status"] == "success"
    assert "Resumo da Wikipedia" in result["data_compressed"]
