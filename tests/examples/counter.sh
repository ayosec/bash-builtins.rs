#!/bin/bash

set -euo pipefail

load_example counter

echo .
counter --help || echo failed
echo .
help counter
echo .

counter -X || echo failed

counter
counter bad || echo failed
counter
counter -r
counter
counter -s -10
counter
