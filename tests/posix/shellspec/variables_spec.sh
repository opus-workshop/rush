#!/bin/sh
# ShellSpec test for POSIX variables and expansion
# Tests parameter expansion, special variables, arithmetic

Describe 'POSIX Variables and Expansion'
  Include ./spec_helper.sh

  Describe 'variable assignment'
    It 'assigns and retrieves variables'
      When call rush_c "FOO=bar && echo \$FOO"
      The output should equal "bar"
      The status should be success
    End

    It 'assigns multiple variables'
      When call rush_c "A=1 B=2 C=3 && echo \$A \$B \$C"
      The output should equal "1 2 3"
      The status should be success
    End

    It 'handles empty value assignment'
      When call rush_c "FOO= && echo \"x\${FOO}y\""
      The output should equal "xy"
      The status should be success
    End

    It 'preserves whitespace in values'
      When call rush_c "FOO='a  b' && echo \"\$FOO\""
      The output should equal "a  b"
      The status should be success
    End
  End

  Describe 'parameter expansion'
    It 'expands simple parameters'
      When call rush_c "FOO=bar && echo \$FOO"
      The output should equal "bar"
      The status should be success
    End

    It 'expands braced parameters'
      When call rush_c "FOO=bar && echo \${FOO}"
      The output should equal "bar"
      The status should be success
    End

    It 'handles unset variables as empty'
      When call rush_c "echo \$UNSET"
      The output should equal ""
      The status should be success
    End

    It 'supports ${var:-default}'
      When call rush_c "echo \${UNSET:-default}"
      The output should equal "default"
      The status should be success
    End

    It 'supports ${var:=default}'
      When call rush_c "echo \${UNSET:=value}; echo \$UNSET"
      The output should include "value"
      The status should be success
    End

    It 'supports ${var:?error}'
      When call rush_c "echo \${UNSET:?missing}"
      The status should be failure
    End

    It 'supports ${var:+alternate}'
      When call rush_c "FOO=bar && echo \${FOO:+alternate}"
      The output should equal "alternate"
      The status should be success
    End

    It 'supports ${#var} for string length'
      When call rush_c "FOO=hello && echo \${#FOO}"
      The output should equal "5"
      The status should be success
    End

    It 'supports ${var%pattern} for suffix removal'
      When call rush_c "FOO=file.txt && echo \${FOO%.txt}"
      The output should equal "file"
      The status should be success
    End

    It 'supports ${var%%pattern} for greedy suffix removal'
      When call rush_c "FOO=file.tar.gz && echo \${FOO%%.*}"
      The output should equal "file"
      The status should be success
    End

    It 'supports ${var#pattern} for prefix removal'
      When call rush_c "FOO=/path/to/file && echo \${FOO#*/}"
      The output should equal "path/to/file"
      The status should be success
    End

    It 'supports ${var##pattern} for greedy prefix removal'
      When call rush_c "FOO=/path/to/file && echo \${FOO##*/}"
      The output should equal "file"
      The status should be success
    End
  End

  Describe 'special variables'
    It '$$ expands to process ID'
      When call rush_c "echo \$\$ | grep -E '^[0-9]+$'"
      The status should be success
    End

    It '$? expands to last exit code'
      When call rush_c "true && echo \$?"
      The output should equal "0"
      The status should be success
    End

    It '$! expands to last background PID'
      When call rush_c "sleep 0.1 & echo \$! | grep -E '^[0-9]+$'"
      The status should be success
    End

    It '$# expands to positional parameter count'
      When call rush_c "set -- a b c && echo \$#"
      The output should equal "3"
      The status should be success
    End

    It '$* expands to all positional parameters'
      When call rush_c "set -- a b c && echo \$*"
      The output should equal "a b c"
      The status should be success
    End

    It '$@ expands to all positional parameters separately'
      When call rush_c "set -- a b c && for x in \"\$@\"; do echo \$x; done"
      The output should include "a"
      The output should include "b"
      The output should include "c"
      The status should be success
    End

    It '$0 expands to shell name'
      When call rush_c "echo \$0"
      The status should be success
    End

    It '$1, $2, etc. expand to positional parameters'
      When call rush_c "set -- a b c && echo \$1 \$2 \$3"
      The output should equal "a b c"
      The status should be success
    End

    It '$- expands to shell options'
      When call rush_c "set -e && echo \$- | grep e"
      The status should be success
    End
  End

  Describe 'command substitution'
    It 'expands $(command)'
      When call rush_c "echo \$(echo test)"
      The output should equal "test"
      The status should be success
    End

    It 'expands `command` (backticks)'
      When call rush_c "echo \`echo test\`"
      The output should equal "test"
      The status should be success
    End

    It 'handles nested command substitution with $()'
      When call rush_c "echo \$(echo \$(echo test))"
      The output should equal "test"
      The status should be success
    End

    It 'preserves exit code'
      When call rush_c "\$(false); echo \$?"
      The output should equal "1"
      The status should be success
    End
  End

  Describe 'arithmetic expansion'
    It 'evaluates $((expr))'
      When call rush_c "echo \$((2 + 2))"
      The output should equal "4"
      The status should be success
    End

    It 'supports addition'
      When call rush_c "echo \$((5 + 3))"
      The output should equal "8"
      The status should be success
    End

    It 'supports subtraction'
      When call rush_c "echo \$((10 - 3))"
      The output should equal "7"
      The status should be success
    End

    It 'supports multiplication'
      When call rush_c "echo \$((4 * 5))"
      The output should equal "20"
      The status should be success
    End

    It 'supports division'
      When call rush_c "echo \$((20 / 4))"
      The output should equal "5"
      The status should be success
    End

    It 'supports modulo'
      When call rush_c "echo \$((17 % 5))"
      The output should equal "2"
      The status should be success
    End

    It 'supports parentheses for precedence'
      When call rush_c "echo \$(( (2 + 3) * 4 ))"
      The output should equal "20"
      The status should be success
    End

    It 'supports variable references in arithmetic'
      When call rush_c "X=5 && echo \$((X * 2))"
      The output should equal "10"
      The status should be success
    End

    It 'supports comparison operators'
      When call rush_c "echo \$((5 > 3))"
      The output should equal "1"
      The status should be success
    End

    It 'supports logical operators'
      When call rush_c "echo \$((1 && 1))"
      The output should equal "1"
      The status should be success
    End
  End

  Describe 'quoting'
    It 'single quotes preserve literal values'
      When call rush_c "echo 'test \$FOO \$(echo x)'"
      The output should equal "test \$FOO \$(echo x)"
      The status should be success
    End

    It 'double quotes allow expansion'
      When call rush_c "FOO=bar && echo \"test \$FOO\""
      The output should equal "test bar"
      The status should be success
    End

    It 'backslash escapes in double quotes'
      When call rush_c "echo \"test\\\"quote\""
      The output should equal "test\"quote"
      The status should be success
    End

    It 'backslash escapes special characters'
      When call rush_c "echo test\\ space"
      The output should equal "test space"
      The status should be success
    End

    It 'preserves whitespace in double quotes'
      When call rush_c "echo \"a  b  c\""
      The output should equal "a  b  c"
      The status should be success
    End
  End

  Describe 'word splitting'
    It 'splits unquoted expansions on IFS'
      When call rush_c "FOO='a b c' && for x in \$FOO; do echo \$x; done"
      The output should include "a"
      The output should include "b"
      The output should include "c"
      The status should be success
    End

    It 'does not split quoted expansions'
      When call rush_c "FOO='a b c' && for x in \"\$FOO\"; do echo \$x; done"
      The output should equal "a b c"
      The status should be success
    End

    It 'respects IFS variable'
      When call rush_c "IFS=: && FOO='a:b:c' && for x in \$FOO; do echo \$x; done"
      The output should include "a"
      The output should include "b"
      The output should include "c"
      The status should be success
    End
  End

  Describe 'pathname expansion (globbing)'
    It 'expands * wildcard'
      When call rush_c "cd /tmp && echo rush_test_* 2>/dev/null || echo no_match"
      The status should be success
    End

    It 'expands ? wildcard'
      When call rush_c "echo /tm?"
      The output should equal "/tmp"
      The status should be success
    End

    It 'expands [...] character class'
      When call rush_c "echo /tm[p]"
      The output should equal "/tmp"
      The status should be success
    End

    It 'does not expand in single quotes'
      When call rush_c "echo '*'"
      The output should equal "*"
      The status should be success
    End

    It 'does not expand in double quotes'
      When call rush_c "echo \"*\""
      The output should equal "*"
      The status should be success
    End
  End

  Describe 'tilde expansion'
    It 'expands ~ to HOME'
      When call rush_c "HOME=/tmp && echo ~"
      The output should equal "/tmp"
      The status should be success
    End

    It 'expands ~/path'
      When call rush_c "HOME=/tmp && echo ~/test"
      The output should equal "/tmp/test"
      The status should be success
    End

    It 'does not expand in quotes'
      When call rush_c "echo '~'"
      The output should equal "~"
      The status should be success
    End
  End

  Describe 'environment variables'
    It 'exports variables to child processes'
      When call rush_c "export FOO=bar && sh -c 'echo \$FOO'"
      The output should equal "bar"
      The status should be success
    End

    It 'inherits environment from parent'
      When call rush_c "echo \$PATH"
      The status should be success
    End

    It 'sets environment for single command'
      When call rush_c "FOO=bar sh -c 'echo \$FOO'"
      The output should equal "bar"
      The status should be success
    End
  End
End
