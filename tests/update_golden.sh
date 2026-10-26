#!/bin/sh
# Regenerate expected outputs (golden files) for the conformance corpus using node.
set -e
cd "$(dirname "$0")/conformance"
for f in *.js; do
  node --expose-gc "$f" > "${f%.js}.out"
done
echo "updated $(ls *.out | wc -l) golden files"
