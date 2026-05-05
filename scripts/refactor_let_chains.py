#!/usr/bin/env python3
"""
Let Chains Refactoring Helper Script

This script helps identify and optionally refactor Rust 2024 let chains
to Rust 2021 compatible nested if statements.

Usage:
    python3 refactor_let_chains.py --dry-run src/adblocker.rs
    python3 refactor_let_chains.py --apply src/adblocker.rs
"""

import re
import sys
import argparse
from pathlib import Path
from typing import List, Tuple

# Pattern to match let chains in if statements
# Matches: if let Ok(x) = expr && let Ok(y) = expr && let Some(z) = expr
LET_CHAIN_PATTERN = re.compile(
    r'(if\s+)?let\s+(\w+)\s*=\s*(.+?)\s+&&\s+let\s+(\w+)\s*=\s*(.+?)(?=\s+&&\s+let|\s*\{|\s*if)',
    re.MULTILINE | re.DOTALL
)

def find_let_chains(content: str) -> List[Tuple[int, str]]:
    """Find all let chains in the content."""
    matches = []
    lines = content.split('\n')
    
    for i, line in enumerate(lines, 1):
        if '&& let' in line:
            matches.append((i, line.strip()))
    
    return matches

def refactor_let_chain(line: str, indent: str) -> str:
    """
    Refactor a single let chain line to nested if.
    
    Before: if let Ok(res) = call() && let Ok(json) = res.json() {
    After:  if let Ok(res) = call() {
                if let Ok(json) = res.json() {
    """
    # Split by && let
    parts = line.split(' && let ')
    
    if len(parts) <= 1:
        return line  # No chains found
    
    result = []
    for i, part in enumerate(parts):
        current_indent = indent + ('    ' * i)
        
        if i == 0:
            # First part keeps the 'if'
            result.append(f"{current_indent}if let {part.strip()} {{")
        else:
            # Subsequent parts become nested if
            result.append(f"{current_indent}if let {part.strip()} {{")
    
    return '\n'.join(result)

def process_file(filepath: Path, dry_run: bool = True) -> Tuple[int, str]:
    """Process a single file and return (changes_count, new_content)."""
    content = filepath.read_text()
    lines = content.split('\n')
    
    changes = 0
    new_lines = []
    skip_next_brace = False
    
    i = 0
    while i < len(lines):
        line = lines[i]
        
        # Check if this line has let chains
        if '&& let' in line and ('if let' in line or lines[i-1].strip().endswith('if let') if i > 0 else False):
            # Get indentation
            indent = len(line) - len(line.lstrip())
            indent_str = ' ' * indent
            
            # Check if this is a multi-line let chain
            if line.strip().endswith('{'):
                # Multi-line chain starting
                chain_lines = [line]
                j = i + 1
                while j < len(lines) and '&& let' in lines[j]:
                    chain_lines.append(lines[j])
                    j += 1
                
                # Refactor the chain
                full_chain = ' '.join(chain_lines)
                refactored = refactor_let_chain(full_chain.replace('{', ''), indent_str)
                new_lines.append(refactored)
                changes += 1
                i = j  # Skip processed lines
                continue
            else:
                # Single line chain
                refactored = refactor_let_chain(line.rstrip('{').strip(), indent_str)
                new_lines.append(refactored)
                changes += 1
        else:
            new_lines.append(line)
        
        i += 1
    
    return changes, '\n'.join(new_lines)

def main():
    parser = argparse.ArgumentParser(description='Refactor Rust let chains to nested if')
    parser.add_argument('files', nargs='+', help='Rust files to process')
    parser.add_argument('--dry-run', action='store_true', help='Show changes without applying')
    parser.add_argument('--apply', action='store_true', help='Apply changes to files')
    
    args = parser.parse_args()
    
    total_changes = 0
    
    for filepath_str in args.files:
        filepath = Path(filepath_str)
        if not filepath.exists():
            print(f"❌ File not found: {filepath}")
            continue
        
        print(f"\n📄 Processing: {filepath}")
        
        # Find let chains
        content = filepath.read_text()
        chains = find_let_chains(content)
        
        if not chains:
            print(f"✅ No let chains found")
            continue
        
        print(f"📍 Found {len(chains)} let chain(s):")
        for line_num, line in chains:
            print(f"   Line {line_num}: {line[:80]}...")
        
        # Process file
        changes, new_content = process_file(filepath)
        
        if changes == 0:
            print(f"✅ No changes needed")
            continue
        
        print(f"\n🔧 Would refactor {changes} let chain(s)")
        
        if args.apply:
            filepath.write_text(new_content)
            print(f"✅ Applied changes to {filepath}")
            total_changes += changes
        elif args.dry_run:
            print(f"📋 Dry run - no changes applied")
    
    print(f"\n{'='*60}")
    print(f"Total let chains found: {total_changes}")
    if total_changes > 0 and not args.apply:
        print("💡 Run with --apply to apply changes")

if __name__ == '__main__':
    main()
