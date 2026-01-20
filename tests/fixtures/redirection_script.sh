#!/usr/bin/env bash
# Test script with redirections
echo "redirect test" > /tmp/rush_redirect_test.txt
cat /tmp/rush_redirect_test.txt
echo "append test" >> /tmp/rush_redirect_test.txt
cat /tmp/rush_redirect_test.txt
rm -f /tmp/rush_redirect_test.txt
