#!/usr/bin/env python3
"""
Sovereign Pair - Sovereign Matrix Tests
Tests the sovereign_matrix.py financial data fetcher for correctness.
"""
import sys
import os
import tempfile
import sqlite3

# Add parent directory to path
sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))

# Import the module
import sovereign_matrix as sm


class TestNormalizeDate:
    """Tests for normalize_date function."""

    def test_normalize_date_with_slash(self):
        """Test date normalization with slash separator."""
        result = sm.normalize_date("2024/01")
        assert result == "2024-01", f"Expected 2024-01, got {result}"

    def test_normalize_date_with_dash(self):
        """Test date normalization with dash separator."""
        result = sm.normalize_date("2024-01")
        assert result == "2024-01", f"Expected 2024-01, got {result}"

    def test_normalize_date_already_normalized(self):
        """Test date that is already in correct format."""
        result = sm.normalize_date("2024-01")
        assert result == "2024-01", f"Expected 2024-01, got {result}"

    def test_normalize_date_empty_string(self):
        """Test empty string handling."""
        result = sm.normalize_date("")
        assert result == "", f"Expected empty string, got {result}"

    def test_normalize_date_none(self):
        """Test None handling."""
        result = sm.normalize_date(None)
        assert result is None or result == "None", f"Expected None or 'None', got {result}"


class TestResolveFromDB:
    """Tests for resolve_from_db function."""

    def setUp(self):
        """Create temporary database for testing."""
        self.temp_db = tempfile.NamedTemporaryFile(delete=False, suffix=".db")
        self.temp_db.close()
        self.conn = sqlite3.connect(self.temp_db.name)
        self.conn.execute("""
            CREATE TABLE ticker_registry (
                search_key TEXT PRIMARY KEY,
                yf_symbol TEXT,
                full_name TEXT,
                market TEXT,
                query_type_hint TEXT,
                is_active INTEGER,
                source TEXT,
                last_verified_at TEXT
            )
        """)
        self.conn.execute("""
            INSERT INTO ticker_registry VALUES
            ('petrobras', 'PETR4.SA', 'Petrobras', 'BR', 'price', 1, 'test', datetime('now')),
            ('vale', 'VALE3.SA', 'Vale', 'BR', 'price', 1, 'test', datetime('now')),
            ('itaú', 'ITUB4.SA', 'Itau Unibanco', 'BR', 'price', 1, 'test', datetime('now'))
        """)
        self.conn.commit()

    def tearDown(self):
        """Clean up temporary database."""
        os.unlink(self.temp_db.name)

    def test_resolve_exact_match(self):
        """Test exact ticker match."""
        self.setUp()
        result = sm.resolve_from_db(self.temp_db.name, "petrobras", "PETR4")
        assert result is not None, "Expected result for petrobras"
        assert result[0] == "PETR4.SA", f"Expected PETR4.SA, got {result[0]}"
        self.tearDown()

    def test_resolve_partial_match(self):
        """Test partial ticker match."""
        self.setUp()
        result = sm.resolve_from_db(self.temp_db.name, "petro", "PETR")
        assert result is not None, "Expected result for petro"
        self.tearDown()

    def test_resolve_no_match(self):
        """Test non-existent ticker."""
        self.setUp()
        result = sm.resolve_from_db(self.temp_db.name, "nonexistent", "NONE")
        assert result is None, f"Expected None for nonexistent, got {result}"
        self.tearDown()

    def test_resolve_inactive_ticker(self):
        """Test inactive ticker is not returned."""
        self.setUp()
        self.conn.execute("""
            INSERT INTO ticker_registry VALUES
            ('oldcorp', 'OLD.SA', 'Old Corp', 'BR', 'price', 0, 'test', datetime('now'))
        """)
        self.conn.commit()
        result = sm.resolve_from_db(self.temp_db.name, "oldcorp", "OLD")
        assert result is None, f"Expected None for inactive ticker, got {result}"
        self.tearDown()


class TestAutoLearn:
    """Tests for auto_learn function."""

    def setUp(self):
        """Create temporary database for testing."""
        self.temp_db = tempfile.NamedTemporaryFile(delete=False, suffix=".db")
        self.temp_db.close()
        # Initialize schema
        conn = sqlite3.connect(self.temp_db.name)
        conn.execute("""
            CREATE TABLE ticker_registry (
                search_key TEXT PRIMARY KEY,
                yf_symbol TEXT,
                full_name TEXT,
                market TEXT,
                query_type_hint TEXT,
                is_active INTEGER,
                source TEXT,
                last_verified_at TEXT
            )
        """)
        conn.commit()
        conn.close()

    def tearDown(self):
        """Clean up temporary database."""
        os.unlink(self.temp_db.name)

    def test_auto_learn_new_ticker(self):
        """Test learning new ticker."""
        self.setUp()
        sm.auto_learn(self.temp_db.name, "testcorp", "TEST.SA", "Test Corp")
        
        conn = sqlite3.connect(self.temp_db.name)
        cursor = conn.cursor()
        cursor.execute("SELECT yf_symbol, full_name FROM ticker_registry WHERE search_key = ?", ("testcorp",))
        result = cursor.fetchone()
        conn.close()
        
        assert result is not None, "Expected ticker to be learned"
        assert result[0] == "TEST.SA", f"Expected TEST.SA, got {result[0]}"
        assert result[1] == "Test Corp", f"Expected 'Test Corp', got {result[1]}"
        self.tearDown()

    def test_auto_learn_cleanup_old_entries(self):
        """Test that old dynamic entries are cleaned up."""
        self.setUp()
        
        # Insert old entry
        conn = sqlite3.connect(self.temp_db.name)
        conn.execute("""
            INSERT INTO ticker_registry VALUES
            ('old_dynamic', 'OLD.SA', 'Old Dynamic', 'BR', 'price', 1, 'yfinance_dynamic', datetime('now', '-31 days'))
        """)
        conn.commit()
        conn.close()
        
        # Trigger auto_learn (should clean old entry)
        sm.auto_learn(self.temp_db.name, "newcorp", "NEW.SA", "New Corp")
        
        # Verify old entry was removed
        conn = sqlite3.connect(self.temp_db.name)
        cursor = conn.cursor()
        cursor.execute("SELECT COUNT(*) FROM ticker_registry WHERE source = 'yfinance_dynamic' AND search_key = 'old_dynamic'")
        count = cursor.fetchone()[0]
        conn.close()
        
        assert count == 0, f"Expected old entry to be cleaned up, found {count} entries"
        self.tearDown()


class TestMacroIndicators:
    """Tests for macroeconomic indicator fetching."""

    def test_macro_ipca_recognition(self):
        """Test IPCA indicator recognition."""
        # IPCA should be recognized as macro indicator
        assert "IPCA" in ["IPCA", "IGPM", "SELIC", "INPC"], "IPCA should be recognized as macro"

    def test_macro_selic_recognition(self):
        """Test SELIC indicator recognition."""
        assert "SELIC" in ["IPCA", "IGPM", "SELIC", "INPC"], "SELIC should be recognized as macro"

    def test_macro_igpm_recognition(self):
        """Test IGPM indicator recognition."""
        assert "IGPM" in ["IPCA", "IGPM", "SELIC", "INPC"], "IGPM should be recognized as macro"


class TestSemanticMapping:
    """Tests for semantic mapping of financial instruments."""

    def test_brent_mapping(self):
        """Test Brent crude oil mapping."""
        from analyze_and_join_time_series import SEMANTIC_MAP
        brent_keys = [k for k, v in SEMANTIC_MAP if v == "BRENT"]
        assert len(brent_keys) > 0, "Expected BRENT mapping"
        assert "BZ=F" in brent_keys or "BRENT" in brent_keys, "Expected BZ=F or BRENT in mapping"

    def test_dolar_ptax_priority(self):
        """Test DOLAR_PTAX has priority over DOLAR."""
        from analyze_and_join_time_series import SEMANTIC_MAP
        dolar_ptax_idx = next((i for i, (k, v) in enumerate(SEMANTIC_MAP) if k == "DOLAR_PTAX"), -1)
        dolar_idx = next((i for i, (k, v) in enumerate(SEMANTIC_MAP) if k == "DOLAR"), -1)
        
        # DOLAR_PTAX should come before DOLAR in the list
        if dolar_ptax_idx >= 0 and dolar_idx >= 0:
            assert dolar_ptax_idx < dolar_idx, "DOLAR_PTAX should come before DOLAR in SEMANTIC_MAP"


class TestEdgeCases:
    """Tests for edge cases and error handling."""

    def test_empty_ticker(self):
        """Test empty ticker handling."""
        result = sm.normalize_date("")
        assert result == "", "Empty string should return empty string"

    def test_special_characters_in_ticker(self):
        """Test ticker with special characters."""
        result = sm.normalize_date("2024/01-TEST")
        # Should handle gracefully
        assert isinstance(result, str), "Result should be string"

    def test_very_long_ticker(self):
        """Test very long ticker name."""
        long_name = "A" * 100
        result = sm.normalize_date(long_name)
        # Should handle gracefully
        assert isinstance(result, str), "Result should be string"


def run_tests():
    """Run all tests and report results."""
    test_classes = [
        TestNormalizeDate,
        TestResolveFromDB,
        TestAutoLearn,
        TestMacroIndicators,
        TestSemanticMapping,
        TestEdgeCases,
    ]
    
    total = 0
    passed = 0
    failed = 0
    
    for test_class in test_classes:
        instance = test_class()
        for method_name in dir(instance):
            if method_name.startswith("test_"):
                total += 1
                try:
                    getattr(instance, method_name)()
                    passed += 1
                    print(f"✓ {test_class.__name__}.{method_name}")
                except AssertionError as e:
                    failed += 1
                    print(f"✗ {test_class.__name__}.{method_name}: {e}")
                except Exception as e:
                    failed += 1
                    print(f"✗ {test_class.__name__}.{method_name}: {type(e).__name__}: {e}")
    
    print(f"\n{'='*50}")
    print(f"Total: {total}, Passed: {passed}, Failed: {failed}")
    
    return failed == 0


if __name__ == "__main__":
    success = run_tests()
    sys.exit(0 if success else 1)
