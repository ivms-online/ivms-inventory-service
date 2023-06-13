##
# This file is part of the IVMS Online.
#
# @copyright 2023 © by Rafał Wrzeszcz - Wrzasq.pl.
##

Feature: Inventory management

    Scenario: Listing inventory
        Given There is an inventory "test4" of type "pc" for vessel "00000000-0000-0000-0000-00000000000a" of customer "00000000-0000-0000-0000-00000000000b" with serial number "qwerty", AWS instance ID "abcd" and creation date "2011-01-30T14:58:00+01:00"
        And There is an inventory "test5" of type "pc" for vessel "00000000-0000-0000-0000-00000000000a" of customer "00000000-0000-0000-0000-00000000000b" with serial number "qwertu", AWS instance ID "abce" and creation date "2015-07-02T03:20:00+02:00"
        When I list inventory for vessel "00000000-0000-0000-0000-00000000000a" of customer "00000000-0000-0000-0000-00000000000b"
        Then I can read list of 2 inventories
        And Inventory at position 0 has ID "test4" and type "pc"
        And Inventory at position 1 has ID "test5" and type "pc"

    Scenario: Listing inventory next page
        Given There is an inventory "test6" of type "pc" for vessel "00000000-0000-0000-0000-00000000000c" of customer "00000000-0000-0000-0000-00000000000d" with serial number "qwerti", AWS instance ID "abcf" and creation date "2017-11-11T16:00:00+02:00"
        And There is an inventory "test7" of type "pc" for vessel "00000000-0000-0000-0000-00000000000c" of customer "00000000-0000-0000-0000-00000000000d" with serial number "qwerto", AWS instance ID "abcg" and creation date "2009-03-23T10:00:00+02:00"
        When I list inventory for vessel "00000000-0000-0000-0000-00000000000c" of customer "00000000-0000-0000-0000-00000000000d" with page token "pc:test6"
        Then I can read list of 1 inventories
        And Inventory at position 0 has ID "test7" and type "pc"
