#!/bin/bash
# Generate custom index.html for GitHub Pages rustdoc documentation
# This script parses Cargo.toml to detect workspace members and creates
# a navigation page for all crates.

set -euo pipefail

# Detect script directory for relative path resolution
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Change to project root
cd "$PROJECT_ROOT"

# Ensure target/doc directory exists
if [ ! -d "target/doc" ]; then
    echo "Error: target/doc directory not found. Run 'cargo doc' first."
    exit 1
fi

# Parse Cargo.toml to extract workspace members
# This reads the [workspace] section and extracts member paths
MEMBERS=()
if [ -f "Cargo.toml" ]; then
    # Extract members array from [workspace] section
    # Matches lines like: "rust/engine", "rust/cli", "rust/web"
    in_members=false
    while IFS= read -r line; do
        # Check if we're in the members array
        if [[ "$line" =~ ^[[:space:]]*members[[:space:]]*=[[:space:]]*\[ ]]; then
            in_members=true
        fi

        # If we're in members array, extract paths
        if [ "$in_members" = true ]; then
            # Extract quoted strings (e.g., "rust/engine")
            if [[ "$line" =~ \"([^\"]+)\" ]]; then
                member_path="${BASH_REMATCH[1]}"
                # Extract last directory component (e.g., "rust/engine" -> "engine")
                base_name=$(basename "$member_path")
                MEMBERS+=("$base_name")
            fi

            # Check if we've reached the end of the array
            if [[ "$line" =~ \] ]]; then
                in_members=false
            fi
        fi
    done < Cargo.toml
fi

# Generate HTML index page
cat > target/doc/index.html << 'EOF'
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>Axiomind Documentation</title>
  <style>
    * {
      margin: 0;
      padding: 0;
      box-sizing: border-box;
    }
    body {
      font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif;
      line-height: 1.6;
      color: #333;
      background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
      min-height: 100vh;
      padding: 20px;
    }
    .container {
      max-width: 900px;
      margin: 0 auto;
      background: white;
      border-radius: 12px;
      box-shadow: 0 20px 60px rgba(0, 0, 0, 0.3);
      overflow: hidden;
    }
    header {
      background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
      color: white;
      padding: 40px 30px;
      text-align: center;
    }
    h1 {
      font-size: 2.5em;
      font-weight: 700;
      margin-bottom: 10px;
    }
    .subtitle {
      font-size: 1.2em;
      opacity: 0.9;
    }
    .content {
      padding: 40px 30px;
    }
    .intro {
      font-size: 1.1em;
      margin-bottom: 30px;
      color: #555;
    }
    .crates-list {
      list-style: none;
    }
    .crate-item {
      margin: 15px 0;
      border-left: 4px solid #667eea;
      transition: all 0.3s ease;
    }
    .crate-item:hover {
      border-left-color: #764ba2;
      transform: translateX(5px);
    }
    .crate-link {
      display: block;
      padding: 20px 25px;
      background: #f8f9fa;
      color: #667eea;
      text-decoration: none;
      font-size: 1.3em;
      font-weight: 600;
      transition: all 0.3s ease;
      border-radius: 4px;
    }
    .crate-link:hover {
      background: #667eea;
      color: white;
    }
    .crate-description {
      display: block;
      font-size: 0.85em;
      font-weight: normal;
      color: #666;
      margin-top: 5px;
    }
    .crate-link:hover .crate-description {
      color: rgba(255, 255, 255, 0.9);
    }
    footer {
      padding: 20px 30px;
      background: #f8f9fa;
      text-align: center;
      color: #666;
      font-size: 0.9em;
    }
    footer a {
      color: #667eea;
      text-decoration: none;
    }
    footer a:hover {
      text-decoration: underline;
    }
    @media (max-width: 768px) {
      body {
        padding: 10px;
      }
      h1 {
        font-size: 2em;
      }
      .content {
        padding: 30px 20px;
      }
      .crate-link {
        font-size: 1.1em;
        padding: 15px 20px;
      }
    }
  </style>
</head>
<body>
  <div class="container">
    <header>
      <h1>Axiomind API Documentation</h1>
      <p class="subtitle">Poker Game Engine and AI Training Platform</p>
    </header>
    <div class="content">
      <p class="intro">
        Welcome to the Axiomind API documentation. This project provides a deterministic poker game engine,
        comprehensive logging, and infrastructure for training and evaluating poker AI agents.
      </p>
      <p class="intro">Select a crate to view its documentation:</p>
      <ul class="crates-list">
EOF

# Add links for each detected crate
for member in "${MEMBERS[@]}"; do
    # Convert member name to crate name convention (e.g., "engine" -> "axm_engine")
    crate_name="axm_${member}"

    # Only add link if the crate documentation directory exists
    if [ -d "target/doc/${crate_name}" ]; then
        # Determine description based on crate name
        case "$member" in
            engine)
                description="Core game engine library - Game rules, state management, and hand evaluation"
                ;;
            cli)
                description="Command-line interface - Simulation, statistics, and batch operations"
                ;;
            web)
                description="Web server - Real-time game streaming and interactive UI"
                ;;
            *)
                description="${member^} crate"
                ;;
        esac

        echo "        <li class=\"crate-item\">" >> target/doc/index.html
        echo "          <a href=\"${crate_name}/index.html\" class=\"crate-link\">" >> target/doc/index.html
        echo "            ${crate_name}" >> target/doc/index.html
        echo "            <span class=\"crate-description\">${description}</span>" >> target/doc/index.html
        echo "          </a>" >> target/doc/index.html
        echo "        </li>" >> target/doc/index.html
    fi
done

# Close HTML document
cat >> target/doc/index.html << 'EOF'
      </ul>
    </div>
    <footer>
      <p>Generated by <a href="https://doc.rust-lang.org/rustdoc/" target="_blank">rustdoc</a></p>
      <p>Project: <a href="https://github.com/omoch1-2357/Axiomind" target="_blank">Axiomind on GitHub</a></p>
    </footer>
  </div>
</body>
</html>
EOF

echo "✓ Generated target/doc/index.html"
echo "✓ Found ${#MEMBERS[@]} workspace members: ${MEMBERS[*]}"
