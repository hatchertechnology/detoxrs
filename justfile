_default:
    @just --list

# Format all markdown with prettier
fmt:
    npx --yes prettier@3 --write "**/*.md"

# Check markdown formatting without writing (CI-friendly; non-zero exit on drift)
fmt-check:
    npx --yes prettier@3 --check "**/*.md"
