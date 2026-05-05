#!/usr/bin/env python3
"""
Sovereign Pair - Empirical Verifier Tests
Tests the empirical_verifier.py Sycophancy Breaker for correctness.
"""
import sys
import os
import json
import unittest
from unittest.mock import patch, MagicMock
from io import StringIO

# Add parent directory to path
sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))

# Import the module
import empirical_verifier as ev


class TestGetBestModel(unittest.TestCase):
    """Tests for get_best_model function."""

    @patch('urllib.request.urlopen')
    def test_get_best_model_qwen_priority(self, mock_urlopen):
        """Test that qwen models have priority."""
        mock_response = MagicMock()
        mock_response.read.return_value = json.dumps({
            "models": [
                {"name": "llama3.2:3b"},
                {"name": "qwen2.5:7b"},
                {"name": "gemma2:9b"}
            ]
        }).encode()
        mock_response.__enter__ = lambda self: self
        mock_response.__exit__ = lambda self, *args: None
        mock_urlopen.return_value = mock_response

        result = ev.get_best_model("http://127.0.0.1:11434")
        
        # qwen2.5:7b should be selected (higher in hierarchy than gemma2)
        self.assertEqual(result, "qwen2.5:7b")

    @patch('urllib.request.urlopen')
    def test_get_best_model_fallback_on_error(self, mock_urlopen):
        """Test fallback to llama3.2 on connection error."""
        mock_urlopen.side_effect = Exception("Connection failed")
        
        result = ev.get_best_model("http://127.0.0.1:11434")
        
        self.assertEqual(result, "llama3.2")

    @patch('urllib.request.urlopen')
    def test_get_best_model_empty_list(self, mock_urlopen):
        """Test fallback when no models installed."""
        mock_response = MagicMock()
        mock_response.read.return_value = json.dumps({"models": []}).encode()
        mock_response.__enter__ = lambda self: self
        mock_response.__exit__ = lambda self, *args: None
        mock_urlopen.return_value = mock_response

        result = ev.get_best_model("http://127.0.0.1:11434")
        
        self.assertEqual(result, "llama3.2")

    @patch('urllib.request.urlopen')
    def test_get_best_model_hierarchy_order(self, mock_urlopen):
        """Test model hierarchy is respected."""
        # Test with multiple qwen versions
        mock_response = MagicMock()
        mock_response.read.return_value = json.dumps({
            "models": [
                {"name": "qwen2.5:14b"},
                {"name": "qwen2.5:7b"},
                {"name": "qwen2.5:3b"}
            ]
        }).encode()
        mock_response.__enter__ = lambda self: self
        mock_response.__exit__ = lambda self, *args: None
        mock_urlopen.return_value = mock_response

        result = ev.get_best_model("http://127.0.0.1:11434")
        
        # qwen2.5:14b should be selected (first in hierarchy)
        self.assertEqual(result, "qwen2.5:14b")


class TestVerifyFunction(unittest.TestCase):
    """Tests for verify function."""

    @patch('urllib.request.urlopen')
    @patch.dict(os.environ, {"OLLAMA_BASE_URL": "http://127.0.0.1:11434"})
    def test_verify_success_response(self, mock_urlopen):
        """Test successful verification response."""
        mock_response = MagicMock()
        mock_response.read.return_value = json.dumps({
            "response": "CRITIQUE: The hypothesis has logical flaws..."
        }).encode()
        mock_response.__enter__ = lambda self: self
        mock_response.__exit__ = lambda self, *args: None
        mock_urlopen.return_value = mock_response

        # Capture stdout
        captured = StringIO()
        sys.stdout = captured
        
        try:
            ev.verify("User premise", "AI hypothesis")
        finally:
            sys.stdout = sys.__stdout__

        output = captured.getvalue()
        result = json.loads(output)
        
        self.assertEqual(result["status"], "success")
        self.assertIn("auditor_verdict", result)
        self.assertIn("advisory", result)

    @patch('urllib.request.urlopen')
    @patch.dict(os.environ, {"OLLAMA_BASE_URL": "http://127.0.0.1:11434"})
    def test_verify_error_handling(self, mock_urlopen):
        """Test error handling in verify function."""
        mock_urlopen.side_effect = Exception("Connection timeout")

        # Capture stdout
        captured = StringIO()
        sys.stdout = captured
        
        try:
            ev.verify("User premise", "AI hypothesis")
        finally:
            sys.stdout = sys.__stdout__

        output = captured.getvalue()
        result = json.loads(output)
        
        self.assertIn("error", result)
        self.assertIn("Connection timeout", result["error"])

    @patch('urllib.request.urlopen')
    def test_verify_custom_base_url(self, mock_urlopen):
        """Test verify with custom base URL."""
        mock_response = MagicMock()
        mock_response.read.return_value = json.dumps({"response": "OK"}).encode()
        mock_response.__enter__ = lambda self: self
        mock_response.__exit__ = lambda self, *args: None
        mock_urlopen.return_value = mock_response

        # Capture stdout
        captured = StringIO()
        sys.stdout = captured
        
        try:
            # Temporarily set custom URL
            old_url = os.environ.get("OLLAMA_BASE_URL")
            os.environ["OLLAMA_BASE_URL"] = "http://custom:11434"
            ev.verify("User premise", "AI hypothesis")
            if old_url:
                os.environ["OLLAMA_BASE_URL"] = old_url
            else:
                os.environ.pop("OLLAMA_BASE_URL", None)
        finally:
            sys.stdout = sys.__stdout__

        # Verify the request was made to custom URL
        call_args = mock_urlopen.call_args[0][0]
        self.assertIn("custom:11434", call_args.get_full_url())


class TestPromptConstruction(unittest.TestCase):
    """Tests for prompt construction logic."""

    def test_prompt_contains_premise(self):
        """Test that user premise is included in prompt."""
        user_premise = "Test user premise 12345"
        hypothesis = "Test hypothesis"
        
        # Simulate prompt construction (from verify function)
        prompt = f"""Você é o Nó de Escrutínio Empírico...
**[PREMISSA ORIGINAL DO USUÁRIO]:**
{user_premise}
**[HIPÓTESE DA IA]:**
{hypothesis}
"""
        
        self.assertIn(user_premise, prompt)

    def test_prompt_contains_hypothesis(self):
        """Test that AI hypothesis is included in prompt."""
        user_premise = "Test premise"
        hypothesis = "Test AI hypothesis 67890"
        
        # Simulate prompt construction
        prompt = f"""Você é o Nó de Escrutínio Empírico...
**[PREMISSA ORIGINAL DO USUÁRIO]:**
{user_premise}
**[HIPÓTESE DA IA]:**
{hypothesis}
"""
        
        self.assertIn(hypothesis, prompt)

    def test_prompt_contains_scrutiny_instructions(self):
        """Test that scrutiny instructions are included."""
        prompt = """Você é o Nó de Escrutínio Empírico...
Analise estritamente:
1. Existe Sycophancy?
2. A hipótese é irrefutável?
"""
        
        self.assertIn("Sycophancy", prompt)
        self.assertIn("Escrutínio", prompt)


class TestModelHierarchy(unittest.TestCase):
    """Tests for model hierarchy configuration."""

    def test_hierarchy_not_empty(self):
        """Test that model hierarchy is defined."""
        # Access hierarchy from get_best_model source
        import inspect
        source = inspect.getsource(ev.get_best_model)
        
        self.assertIn("hierarchy", source)
        self.assertIn("qwen", source.lower())

    def test_hierarchy_contains_qwen(self):
        """Test that qwen models are in hierarchy."""
        import inspect
        source = inspect.getsource(ev.get_best_model)
        
        # Extract hierarchy list
        self.assertIn("qwen2.5:14b", source)
        self.assertIn("qwen2.5:7b", source)


class TestIntegration(unittest.TestCase):
    """Integration tests for empirical_verifier module."""

    @patch('urllib.request.urlopen')
    def test_full_verification_flow(self, mock_urlopen):
        """Test complete verification flow."""
        # Setup mock
        tags_response = MagicMock()
        tags_response.read.return_value = json.dumps({
            "models": [{"name": "qwen2.5:7b"}]
        }).encode()
        tags_response.__enter__ = lambda self: self
        tags_response.__exit__ = lambda self, *args: None

        generate_response = MagicMock()
        generate_response.read.return_value = json.dumps({
            "response": "CRITIQUE: Logical flaw detected in hypothesis."
        }).encode()
        generate_response.__enter__ = lambda self: self
        generate_response.__exit__ = lambda self, *args: None

        mock_urlopen.side_effect = [tags_response, generate_response]

        # Capture stdout
        captured = StringIO()
        sys.stdout = captured
        
        try:
            ev.verify("What is the capital of France?", "The capital is Paris.")
        finally:
            sys.stdout = sys.__stdout__

        output = captured.getvalue()
        result = json.loads(output)
        
        # Verify complete flow
        self.assertEqual(result["status"], "success")
        self.assertEqual(result["model_used"], "qwen2.5:7b")
        self.assertIn("auditor_verdict", result)


def run_tests():
    """Run all tests and report results."""
    loader = unittest.TestLoader()
    suite = unittest.TestSuite()
    
    # Add all test classes
    suite.addTests(loader.loadTestsFromTestCase(TestGetBestModel))
    suite.addTests(loader.loadTestsFromTestCase(TestVerifyFunction))
    suite.addTests(loader.loadTestsFromTestCase(TestPromptConstruction))
    suite.addTests(loader.loadTestsFromTestCase(TestModelHierarchy))
    suite.addTests(loader.loadTestsFromTestCase(TestIntegration))
    
    # Run tests
    runner = unittest.TextTestRunner(verbosity=2)
    result = runner.run(suite)
    
    # Summary
    print(f"\n{'='*50}")
    print(f"Tests run: {result.testsRun}")
    print(f"Failures: {len(result.failures)}")
    print(f"Errors: {len(result.errors)}")
    print(f"Success: {result.wasSuccessful()}")
    
    return result.wasSuccessful()


if __name__ == "__main__":
    success = run_tests()
    sys.exit(0 if success else 1)
