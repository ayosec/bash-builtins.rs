#!/bin/bash

load_example nonrequiredargs

nonrequiredargs
nonrequiredargs -f
nonrequiredargs -f -b
nonrequiredargs -f -b bar
nonrequiredargs -f 42 -b
nonrequiredargs -f 1 -b foo
