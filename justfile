_default:
    @just --list

# Format all markdown with prettier
fmt:
    npx --yes prettier@3 --write "**/*.md"

# Check markdown formatting without writing (CI-friendly; non-zero exit on drift)
fmt-check:
    npx --yes prettier@3 --check "**/*.md"

# Format specific files: just fmt-file docs/a.md docs/b.md
fmt-file +files:
    npx --yes prettier@3 --ignore-unknown --write {{ files }}

# Check specific files: just fmt-check-file docs/a.md
fmt-check-file +files:
    npx --yes prettier@3 --ignore-unknown --check {{ files }}
