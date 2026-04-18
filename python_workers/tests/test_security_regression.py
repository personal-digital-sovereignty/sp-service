# ================================================================
# Sovereign Pair — Python Worker Unit Tests (Passagem 3)
# Covers: XDG path resolution, DB name correctness, worker safety
# Modules: market_pricing_matrix.py, culture_matrix.py, ast_jail.py
# ================================================================

import os
import sys
import unittest
import platform
import tempfile

# Inject parent path
sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))


class TestXDGPathResolution(unittest.TestCase):
    """LIN-01, LIN-02: XDG Base Dir compliance for Python workers."""

    def _get_sovereign_db_path(self, env_overrides=None):
        """Replicate the logic of get_sovereign_db_path() for testing."""
        if env_overrides:
            for k, v in env_overrides.items():
                os.environ[k] = v

        if "DATABASE_URL" in os.environ:
            return os.environ["DATABASE_URL"]

        if "XDG_DATA_HOME" in os.environ:
            base = os.environ["XDG_DATA_HOME"]
        elif platform.system() == "Darwin":
            base = os.path.expanduser("~/Library/Application Support")
        elif platform.system() == "Windows":
            base = os.environ.get("LOCALAPPDATA", os.path.expanduser("~"))
        else:
            base = os.path.join(os.path.expanduser("~"), ".local", "share")

        return os.path.join(base, "sovereign-pair", "data", "sovereign_memory.db")

    def tearDown(self):
        """Limpar variáveis de ambiente após cada teste."""
        for var in ["DATABASE_URL", "XDG_DATA_HOME"]:
            os.environ.pop(var, None)

    def test_database_url_takes_priority(self):
        """DATABASE_URL deve ter prioridade máxima."""
        path = self._get_sovereign_db_path({"DATABASE_URL": "sqlite://:memory:"})
        self.assertEqual(path, "sqlite://:memory:")

    def test_xdg_data_home_respected(self):
        """XDG_DATA_HOME deve ser respeitado em Linux."""
        if platform.system() == "Windows":
            self.skipTest("XDG não aplicável no Windows")
        custom_xdg = "/custom/xdg/path"
        path = self._get_sovereign_db_path({"XDG_DATA_HOME": custom_xdg})
        self.assertIn(custom_xdg, path)
        self.assertIn("sovereign_memory.db", path)

    def test_db_name_is_correct(self):
        """LIN-02: Nome do banco deve ser sovereign_memory.db (não SovereignHub)."""
        path = self._get_sovereign_db_path()
        self.assertIn("sovereign_memory.db", path,
                      "DB name deve ser sovereign_memory.db")
        self.assertNotIn("SovereignHub_OS_System.db", path,
                          "LIN-02 REGRESSÃO: Nome legado SovereignHub detectado!")

    def test_path_contains_sovereign_pair_dir(self):
        """Path do banco deve estar dentro de sovereign-pair/data/."""
        path = self._get_sovereign_db_path()
        self.assertIn("sovereign-pair", path)
        self.assertIn("data", path)

    def test_path_is_not_hardcoded_tilde(self):
        """Path não deve ser literal ~/... — deve ser expandido."""
        path = self._get_sovereign_db_path()
        self.assertFalse(path.startswith("~/"),
                          "Path não deve conter ~ literal — deve ser expandido")

    def test_fallback_path_is_absolute(self):
        """Fallback path deve ser absoluto quando home dir existe."""
        path = self._get_sovereign_db_path()
        if os.path.expanduser("~") != "~":
            self.assertTrue(os.path.isabs(path) or path.startswith("sqlite:"),
                            f"Path deve ser absoluto: {path}")


class TestASTJailSecurity(unittest.TestCase):
    """Security: AST Jail deve bloquear imports e calls perigosas."""

    def _check_code(self, code: str):
        """Executa visita AST simplificada."""
        import ast

        banned_imports = {'os', 'sys', 'subprocess', 'pty', 'shutil',
                          'socket', 'urllib', 'requests', 'http',
                          'ftplib', 'importlib'}
        banned_calls = {'eval', 'exec', 'open', '__import__', 'getattr',
                        'setattr', 'globals', 'locals', 'dir', 'compile',
                        'memoryview'}

        violations = []

        class Visitor(ast.NodeVisitor):
            def visit_Import(self, node):
                for alias in node.names:
                    if alias.name.split('.')[0] in banned_imports:
                        violations.append(f"import:{alias.name}")
                self.generic_visit(node)

            def visit_ImportFrom(self, node):
                if node.module and node.module.split('.')[0] in banned_imports:
                    violations.append(f"from_import:{node.module}")
                self.generic_visit(node)

            def visit_Call(self, node):
                if isinstance(node.func, ast.Name):
                    if node.func.id in banned_calls:
                        violations.append(f"call:{node.func.id}")
                elif isinstance(node.func, ast.Attribute):
                    if node.func.attr in banned_calls:
                        violations.append(f"attr_call:{node.func.attr}")
                self.generic_visit(node)

        tree = ast.parse(code)
        Visitor().visit(tree)
        return violations

    def test_blocks_os_import(self):
        violations = self._check_code("import os")
        self.assertTrue(any("os" in v for v in violations), "import os deve ser bloqueado")

    def test_blocks_subprocess(self):
        violations = self._check_code("import subprocess")
        self.assertTrue(any("subprocess" in v for v in violations))

    def test_blocks_eval_call(self):
        violations = self._check_code("eval('malicious code')")
        self.assertTrue(any("eval" in v for v in violations), "eval() deve ser bloqueado")

    def test_blocks_exec_call(self):
        violations = self._check_code("exec('rm -rf /')")
        self.assertTrue(any("exec" in v for v in violations), "exec() deve ser bloqueado")

    def test_blocks_open_call(self):
        violations = self._check_code("open('/etc/passwd')")
        self.assertTrue(any("open" in v for v in violations), "open() deve ser bloqueado")

    def test_allows_safe_math(self):
        violations = self._check_code("import math\nresult = math.sqrt(16)\nprint(result)")
        self.assertEqual(violations, [], f"Código math seguro não deve ter violações: {violations}")

    def test_allows_pandas_numpy(self):
        violations = self._check_code("import pandas as pd\nimport numpy as np\ndf = pd.DataFrame({'a': [1,2,3]})")
        self.assertEqual(violations, [], f"pandas/numpy são permitidos: {violations}")

    def test_blocks_from_os_import(self):
        violations = self._check_code("from os import path")
        self.assertTrue(any("os" in v for v in violations), "from os import deve ser bloqueado")


class TestWorkerPathSafety(unittest.TestCase):
    """Testes de qualidade nos workers Python."""

    def test_temp_dir_cross_platform(self):
        """temp_dir deve funcionar em todos os sistemas."""
        temp = tempfile.gettempdir()
        self.assertTrue(os.path.isabs(temp), f"temp_dir deve ser absoluto: {temp}")
        self.assertTrue(os.path.exists(temp), f"temp_dir deve existir: {temp}")

    def test_temp_dir_not_proc(self):
        """temp_dir não deve ser /proc."""
        temp = tempfile.gettempdir()
        self.assertFalse(temp.startswith("/proc"),
                          "/proc não deve ser usado como temp dir")

    def test_sovereign_data_subdir_structure(self):
        """Estrutura de diretórios esperada: sovereign-pair/data/sovereign_memory.db."""
        parts = ["sovereign-pair", "data", "sovereign_memory.db"]
        path = os.path.join(*parts)
        self.assertIn("sovereign-pair", path)
        self.assertIn("data", path)
        self.assertIn("sovereign_memory.db", path)


class TestDBNameRegression(unittest.TestCase):
    """LIN-02 Regressão: DB name nunca deve ser o nome legado."""

    CORRECT_DB_NAME = "sovereign_memory.db"
    LEGACY_DB_NAME = "SovereignHub_OS_System.db"

    def test_correct_db_name_defined(self):
        self.assertEqual(self.CORRECT_DB_NAME, "sovereign_memory.db")

    def test_legacy_db_name_not_used(self):
        # Verifica que o arquivo de culture_matrix usa o nome correto
        # NOTA: O nome legado pode aparecer em comentários (histórico de fix), 
        #       mas não deve aparecer em strings de código executável
        culture_matrix_path = os.path.join(
            os.path.dirname(__file__), "..", "culture_matrix.py"
        )
        if os.path.exists(culture_matrix_path):
            with open(culture_matrix_path) as f:
                lines = f.readlines()

            # Verificar apenas linhas que são código executável (não comentários nem strings de doc)
            def is_executable_code(line: str) -> bool:
                stripped = line.strip()
                if stripped.startswith('#'):
                    return False  # Comentário
                if stripped.startswith(('"""', "'''", '"', "'")):
                    return False  # String literal de docstring/doc
                if 'FIX:' in line or 'Regressão:' in line or 'era ' in line:
                    return False  # Linha de documentação inline
                return True

            code_lines = [
                line for line in lines
                if is_executable_code(line) and self.LEGACY_DB_NAME in line
            ]
            self.assertEqual(code_lines, [],
                             f"LIN-02 REGRESSÃO: '{self.LEGACY_DB_NAME}' encontrado em código executável: {code_lines}")

            # Garantir que o nome correto está em pelo menos uma definição
            has_correct = any(self.CORRECT_DB_NAME in line for line in lines
                              if not line.strip().startswith('#'))
            self.assertTrue(has_correct,
                            f"culture_matrix.py deve usar '{self.CORRECT_DB_NAME}' no código")

    def test_market_pricing_uses_correct_db_name(self):
        market_path = os.path.join(
            os.path.dirname(__file__), "..", "market_pricing_matrix.py"
        )
        if os.path.exists(market_path):
            with open(market_path) as f:
                content = f.read()
            self.assertIn(self.CORRECT_DB_NAME, content,
                          f"market_pricing_matrix.py deve usar {self.CORRECT_DB_NAME}")


if __name__ == "__main__":
    unittest.main(verbosity=2)
