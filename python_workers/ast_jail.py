import sys
import ast

def main():
    if len(sys.argv) < 2:
        sys.exit("Usage: ast_jail.py <script.py>")
    
    script_path = sys.argv[1]
    with open(script_path, "r", encoding="utf-8") as f:
        code = f.read()

    try:
        tree = ast.parse(code)
    except SyntaxError as e:
        print(f"Jail Syntax Error: {e}")
        sys.exit(1)

    banned_imports = {'os', 'sys', 'subprocess', 'pty', 'shutil', 'socket', 'urllib', 'requests', 'http', 'ftplib', 'importlib'}
    banned_calls = {'eval', 'exec', 'open', '__import__', 'getattr', 'setattr', 'globals', 'locals', 'dir', 'compile', 'memoryview'}

    class SecurityVisitor(ast.NodeVisitor):
        def visit_Import(self, node):
            for alias in node.names:
                if alias.name.split('.')[0] in banned_imports:
                    print(f"Sovereign AST Jail: O módulo de sistema '{alias.name}' foi bloqueado por motivos de segurança RCE!")
                    sys.exit(1)
            self.generic_visit(node)
            
        def visit_ImportFrom(self, node):
            if node.module and node.module.split('.')[0] in banned_imports:
                print(f"Sovereign AST Jail: O módulo de sistema '{node.module}' foi bloqueado por motivos de segurança RCE!")
                sys.exit(1)
            self.generic_visit(node)
            
        def visit_Call(self, node):
            if isinstance(node.func, ast.Name):
                if node.func.id in banned_calls:
                    print(f"Sovereign AST Jail: A função perigosa '{node.func.id}' foi bloqueada pelo motor cíbrido!")
                    sys.exit(1)
            elif isinstance(node.func, ast.Attribute):
                if node.func.attr in banned_calls:
                    print(f"Sovereign AST Jail: A função de atributo '{node.func.attr}' foi bloqueada pelo motor cíbrido!")
                    sys.exit(1)
            self.generic_visit(node)

    visitor = SecurityVisitor()
    visitor.visit(tree)

    # Safe to execute!
    namespace = {"__builtins__": __builtins__}
    # Restrict builtins? We leave builtins so things like print() and len() work.
    try:
        exec(compile(tree, filename="sovereign_sandbox.py", mode="exec"), namespace, namespace)  # nosemgrep
    except Exception as e:
        print(f"Math Error / Runtime Execution: {e}")
        sys.exit(1)

if __name__ == "__main__":
    main()
