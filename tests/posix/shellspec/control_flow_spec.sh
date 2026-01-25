#!/bin/sh
# ShellSpec test for POSIX control flow constructs
# Tests if, while, until, for, case

Describe 'POSIX Control Flow'
  Include ./spec_helper.sh

  Describe 'if statement'
    It 'executes then branch when condition is true'
      When call rush_c "if true; then echo yes; fi"
      The output should equal "yes"
      The status should be success
    End

    It 'executes else branch when condition is false'
      When call rush_c "if false; then echo yes; else echo no; fi"
      The output should equal "no"
      The status should be success
    End

    It 'supports elif'
      When call rush_c "if false; then echo 1; elif true; then echo 2; else echo 3; fi"
      The output should equal "2"
      The status should be success
    End

    It 'supports nested if statements'
      When call rush_c "if true; then if true; then echo nested; fi; fi"
      The output should equal "nested"
      The status should be success
    End

    It 'uses command exit code as condition'
      When call rush_c "if test 1 -eq 1; then echo ok; fi"
      The output should equal "ok"
      The status should be success
    End
  End

  Describe 'while loop'
    It 'executes while condition is true'
      When call rush_c "i=0; while [ \$i -lt 3 ]; do echo \$i; i=\$((i+1)); done"
      The output should include "0"
      The output should include "1"
      The output should include "2"
      The status should be success
    End

    It 'does not execute if condition is initially false'
      When call rush_c "while false; do echo should_not_print; done"
      The output should equal ""
      The status should be success
    End

    It 'supports break in while loop'
      When call rush_c "i=0; while true; do echo \$i; [ \$i -eq 2 ] && break; i=\$((i+1)); done"
      The output should include "2"
      The status should be success
    End

    It 'supports continue in while loop'
      When call rush_c "i=0; while [ \$i -lt 3 ]; do i=\$((i+1)); [ \$i -eq 2 ] && continue; echo \$i; done"
      The output should include "1"
      The output should not include "2"
      The output should include "3"
      The status should be success
    End

    It 'supports nested while loops'
      When call rush_c "i=0; while [ \$i -lt 2 ]; do j=0; while [ \$j -lt 2 ]; do echo \$i\$j; j=\$((j+1)); done; i=\$((i+1)); done"
      The output should include "00"
      The output should include "01"
      The output should include "10"
      The output should include "11"
      The status should be success
    End
  End

  Describe 'until loop'
    It 'executes until condition becomes true'
      When call rush_c "i=0; until [ \$i -eq 3 ]; do echo \$i; i=\$((i+1)); done"
      The output should include "0"
      The output should include "1"
      The output should include "2"
      The status should be success
    End

    It 'does not execute if condition is initially true'
      When call rush_c "until true; do echo should_not_print; done"
      The output should equal ""
      The status should be success
    End

    It 'supports break in until loop'
      When call rush_c "i=0; until false; do echo \$i; [ \$i -eq 2 ] && break; i=\$((i+1)); done"
      The output should include "2"
      The status should be success
    End

    It 'supports continue in until loop'
      When call rush_c "i=0; until [ \$i -eq 3 ]; do i=\$((i+1)); [ \$i -eq 2 ] && continue; echo \$i; done"
      The output should include "1"
      The output should not include "2"
      The output should include "3"
      The status should be success
    End
  End

  Describe 'for loop'
    It 'iterates over list'
      When call rush_c "for i in a b c; do echo \$i; done"
      The output should include "a"
      The output should include "b"
      The output should include "c"
      The status should be success
    End

    It 'iterates over positional parameters with for var'
      When call rush_c "set -- x y z && for i; do echo \$i; done"
      The output should include "x"
      The output should include "y"
      The output should include "z"
      The status should be success
    End

    It 'supports break in for loop'
      When call rush_c "for i in 1 2 3 4 5; do echo \$i; [ \$i = 3 ] && break; done"
      The output should include "1"
      The output should include "2"
      The output should include "3"
      The output should not include "4"
      The status should be success
    End

    It 'supports continue in for loop'
      When call rush_c "for i in 1 2 3; do [ \$i = 2 ] && continue; echo \$i; done"
      The output should include "1"
      The output should not include "2"
      The output should include "3"
      The status should be success
    End

    It 'supports nested for loops'
      When call rush_c "for i in a b; do for j in 1 2; do echo \$i\$j; done; done"
      The output should include "a1"
      The output should include "a2"
      The output should include "b1"
      The output should include "b2"
      The status should be success
    End

    It 'handles empty list'
      When call rush_c "for i in; do echo should_not_print; done"
      The output should equal ""
      The status should be success
    End
  End

  Describe 'case statement'
    It 'matches simple patterns'
      When call rush_c "case foo in foo) echo match;; esac"
      The output should equal "match"
      The status should be success
    End

    It 'matches wildcard patterns'
      When call rush_c "case hello in hel*) echo match;; esac"
      The output should equal "match"
      The status should be success
    End

    It 'matches character classes'
      When call rush_c "case a in [abc]) echo match;; esac"
      The output should equal "match"
      The status should be success
    End

    It 'supports multiple patterns per case'
      When call rush_c "case 2 in 1|2|3) echo match;; esac"
      The output should equal "match"
      The status should be success
    End

    It 'matches first matching pattern only'
      When call rush_c "case foo in foo) echo first;; *) echo second;; esac"
      The output should equal "first"
      The status should be success
    End

    It 'supports default pattern'
      When call rush_c "case xyz in abc) echo 1;; *) echo default;; esac"
      The output should equal "default"
      The status should be success
    End

    It 'does nothing if no pattern matches'
      When call rush_c "case xyz in abc) echo match;; esac"
      The output should equal ""
      The status should be success
    End

    It 'supports nested case statements'
      When call rush_c "case a in a) case b in b) echo nested;; esac;; esac"
      The output should equal "nested"
      The status should be success
    End
  End

  Describe '&& (AND) operator'
    It 'executes second command if first succeeds'
      When call rush_c "true && echo yes"
      The output should equal "yes"
      The status should be success
    End

    It 'does not execute second command if first fails'
      When call rush_c "false && echo should_not_print"
      The output should equal ""
      The status should be failure
    End

    It 'chains multiple commands'
      When call rush_c "true && true && echo ok"
      The output should equal "ok"
      The status should be success
    End
  End

  Describe '|| (OR) operator'
    It 'executes second command if first fails'
      When call rush_c "false || echo yes"
      The output should equal "yes"
      The status should be success
    End

    It 'does not execute second command if first succeeds'
      When call rush_c "true || echo should_not_print"
      The output should equal ""
      The status should be success
    End

    It 'chains multiple commands'
      When call rush_c "false || false || echo ok"
      The output should equal "ok"
      The status should be success
    End
  End

  Describe 'combined && and ||'
    It 'handles mixed operators'
      When call rush_c "false || true && echo ok"
      The output should equal "ok"
      The status should be success
    End

    It 'respects operator precedence'
      When call rush_c "true && false || echo ok"
      The output should equal "ok"
      The status should be success
    End
  End

  Describe 'command grouping with { }'
    It 'groups commands'
      When call rush_c "{ echo a; echo b; }"
      The output should include "a"
      The output should include "b"
      The status should be success
    End

    It 'executes in current shell'
      When call rush_c "FOO=bar; { FOO=baz; }; echo \$FOO"
      The output should equal "baz"
      The status should be success
    End
  End

  Describe 'subshell with ( )'
    It 'groups commands in subshell'
      When call rush_c "(echo a; echo b)"
      The output should include "a"
      The output should include "b"
      The status should be success
    End

    It 'isolates variables'
      When call rush_c "FOO=bar; (FOO=baz); echo \$FOO"
      The output should equal "bar"
      The status should be success
    End
  End

  Describe 'command lists'
    It 'executes commands sequentially with ;'
      When call rush_c "echo a; echo b; echo c"
      The output should include "a"
      The output should include "b"
      The output should include "c"
      The status should be success
    End

    It 'executes commands in background with &'
      When call rush_c "sleep 0.1 & echo immediate"
      The output should equal "immediate"
      The status should be success
    End
  End
End
