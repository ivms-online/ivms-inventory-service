##
# This file is part of the IVMS Online.
#
# @copyright 2023 © by Rafał Wrzeszcz - Wrzasq.pl.
##

Feature: Inventory management

    Scenario: Creating inventory
        When I create inventory "test8" of type "pc" for vessel "00000000-0000-0000-0000-000000000001" of customer "00000000-0000-0000-0000-000000000005" with serial number "qwerts" and AWS instance ID "abcj"
        Then I can read inventory key
        And Inventory with that key exists with serial number "qwerts", AWS instance ID "abcj" and creation date
