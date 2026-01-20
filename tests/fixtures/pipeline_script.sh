#!/usr/bin/env bash
# Test script with pipelines
echo "line1" > /tmp/rush_test_pipeline.txt
echo "line2" >> /tmp/rush_test_pipeline.txt
echo "line3" >> /tmp/rush_test_pipeline.txt
cat /tmp/rush_test_pipeline.txt | grep line2
rm -f /tmp/rush_test_pipeline.txt
