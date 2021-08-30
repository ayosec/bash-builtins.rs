#!/bin/bash

set -euo pipefail

load_example varcounter

# Read variables.

varcounter FIRST SECOND THIRD
varcounter FIRST &> /dev/null

echo $FIRST $SECOND $FIRST
echo $SECOND
echo $SECOND $THIRD

SECOND=1000
echo $SECOND $SECOND

unset FIRST THIRD

FIRST=X
echo $FIRST ${THIRD:-NA}

enable -d varcounter

echo $FIRST
echo ${SECOND:-NA}
