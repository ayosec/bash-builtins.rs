#!/bin/bash

set -euo pipefail

load_example upcase

echo .
upcase a bb1 ccc2 dddd
echo .
upcase
echo .
upcase äλ
echo .
