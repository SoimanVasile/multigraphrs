#!/bin/bash

number_of_lines=0
find tests -type f| while read path; do
    (grep -c "" "$path")
done | awk '{sum+=$1;} END {print sum;}'

