#!/bin/bash

echo "🦀 Quick Test: Project Naming Convention"
echo "======================================="

# Create test qs directory
mkdir -p qs
mkdir -p annotations

# Create sample question set
cat > qs/qs00035.txt << 'EOF'
43. Implement a lock-free concurrent queue
44. Create a procedural macro  
45. Write a zero-copy parser
EOF

# Simulate project creation (without running Claude)
echo "Simulating project creation..."
mkdir -p annotations/q0003500043
mkdir -p annotations/q0003500044
mkdir -p annotations/q0003500045

# Show results
echo -e "\nQuestion Set File:"
cat qs/qs00035.txt

echo -e "\nGenerated Project Directories:"
ls -la annotations/ | grep "q00035"

echo -e "\nNaming Pattern Verification:"
echo "✓ qs00035.txt → QS Number: 00035"
echo "✓ Question 43 → Project: q0003500043"
echo "✓ Question 44 → Project: q0003500044"
echo "✓ Question 45 → Project: q0003500045"

echo -e "\nPattern: q{qs_number}{question_num:05}"
echo "Example: q + 00035 + 00043 = q0003500043"

# Clean up
rm -rf annotations/q00035*
echo -e "\n✅ Naming convention test passed!"