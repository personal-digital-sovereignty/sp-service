import pytest
import sys
import os
import json

sys.path.append(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

from academic_matrix import fetch_arxiv, fetch_pubmed

def test_fetch_arxiv_mock(monkeypatch, capsys):
    def mock_urlopen(req, timeout=15):
        class MockResponse:
            def read(self):
                return b'''<?xml version="1.0" encoding="UTF-8"?>
                <feed xmlns="http://www.w3.org/2005/Atom">
                  <entry>
                    <title>Test Paper</title>
                    <summary>Test summary.</summary>
                  </entry>
                </feed>'''
        return MockResponse()
    
    monkeypatch.setattr('urllib.request.urlopen', mock_urlopen)
    
    fetch_arxiv("quantum computing")
    captured = capsys.readouterr()
    result = json.loads(captured.out)
    
    assert result["status"] == "success"
    assert "Test Paper" in result["data_compressed"]

def test_fetch_pubmed_mock(monkeypatch, capsys):
    def mock_urlopen(req, timeout=15):
        class MockResponse:
            def read(self):
                url = req.full_url
                if "esearch" in url:
                    return b'{"esearchresult": {"idlist": ["12345"]}}'
                else:
                    return b'{"result": {"12345": {"title": "PubMed Paper"}}}'
        return MockResponse()
    
    monkeypatch.setattr('urllib.request.urlopen', mock_urlopen)
    
    fetch_pubmed("machine learning")
    captured = capsys.readouterr()
    result = json.loads(captured.out)
    
    assert result["status"] == "success"
    assert "PubMed Paper" in result["data_compressed"]
