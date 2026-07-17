#!/usr/bin/env python3
import json
import subprocess
import sys
import re

# Allowed licenses whitelist (case-insensitive)
ALLOWED_LICENSES = {
    "mit",
    "apache-2.0",
    "bsd-3-clause",
    "bsd-2-clause",
    "cc0-1.0",
    "isc",
    "zlib",
    "unicode-dfs-2016",
    "unicode-3.0",
    "0bsd",
    "openssl",
    "mpl-2.0",
    "bsl-1.0",
}

# Copyleft licenses blacklist (case-insensitive)
BLACKLIST_LICENSES = {
    "gpl",
    "lgpl",
    "agpl",
}

def get_cargo_metadata():
    try:
        result = subprocess.run(
            ["cargo", "metadata", "--format-version", "1"],
            capture_output=True,
            text=True,
            check=True
        )
        return json.loads(result.stdout)
    except Exception as e:
        print(f"Error running cargo metadata: {e}", file=sys.stderr)
        sys.exit(1)

def is_license_safe(license_str):
    """
    Checks if a single license term (e.g. 'MIT', 'GPL-2.0-only') is safe.
    It must match allowed licenses and must not contain any blacklisted patterns.
    """
    lic_lower = license_str.strip().lower()
    
    # Check blacklist first
    for black in BLACKLIST_LICENSES:
        if black in lic_lower:
            return False
            
    # Check whitelist
    for allow in ALLOWED_LICENSES:
        if allow in lic_lower:
            return True
            
    return False

def check_package_license(license_expr):
    """
    Evaluates a license expression (e.g., 'MIT OR Apache-2.0', 'GPL-2.0-only OR BSD-3-Clause')
    and returns True if the expression is acceptable.
    For 'OR' expressions, if at least one choice is safe, the package is considered safe.
    For 'AND' expressions (or simple single licenses), it must be safe.
    """
    if not license_expr:
        return True, "No license specified"

    # Split by ' OR ' (case-insensitive)
    or_choices = re.split(r'\s+[Oo][Rr]\s+', license_expr)
    
    safe_choices = []
    unsafe_choices = []
    
    for choice in or_choices:
        # Strip outer parentheses
        choice_clean = choice.replace("(", "").replace(")", "").strip()
        
        # Handle 'AND' within choice
        if " AND " in choice_clean.upper():
            and_parts = re.split(r'\s+[Aa][Nn][Dd]\s+', choice_clean)
            if all(is_license_safe(part) for part in and_parts):
                safe_choices.append(choice)
            else:
                unsafe_choices.append(choice)
        else:
            if is_license_safe(choice_clean):
                safe_choices.append(choice)
            else:
                unsafe_choices.append(choice)
                
    if safe_choices:
        # At least one OR path is safe, which is sufficient
        return True, None
    else:
        return False, f"None of the license choices {unsafe_choices} are approved or safe"

def main():
    metadata = get_cargo_metadata()
    packages = metadata.get("packages", [])
    
    failed = False
    print("=== License Audit Scanner ===")
    
    for pkg in packages:
        pkg_name = pkg.get("name")
        pkg_version = pkg.get("version")
        license_expr = pkg.get("license")
        
        # Skip workspace member package checks if desired
        if pkg_name in ["mcp-uutils-lib", "mcp-uutils"]:
            continue
            
        is_ok, err_reason = check_package_license(license_expr)
        if not is_ok:
            print(f"ERROR: {pkg_name} ({pkg_version}) - {err_reason} (License expression: '{license_expr}')")
            failed = True
            
    if failed:
        print("\nAudit failed. Unapproved or unsafe licenses detected.", file=sys.stderr)
        sys.exit(1)
    else:
        print("\nLicense audit passed successfully.")
        sys.exit(0)

if __name__ == "__main__":
    main()
