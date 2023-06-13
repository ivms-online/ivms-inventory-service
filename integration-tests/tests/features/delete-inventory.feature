##
# This file is part of the IVMS Online.
#
# @copyright 2023 © by Rafał Wrzeszcz - Wrzasq.pl.
##

Feature: Inventory management

    Scenario: Deleting inventory
        Given There is an inventory "test0" of type "pc" for vessel "00000000-0000-0000-0000-000000000000" of customer "00000000-0000-0000-0000-000000000001" with serial number "qwerta", AWS instance ID "abci" and creation date "2011-01-30T14:58:00+01:00"
        When I delete inventory "test0" of type "pc" for vessel "00000000-0000-0000-0000-000000000000" of customer "00000000-0000-0000-0000-000000000001"
        Then Inventory "test0" of type "pc" for vessel "00000000-0000-0000-0000-000000000000" of customer "00000000-0000-0000-0000-000000000001" does not exist

    Scenario: Deleting non-existing inventory
        Given There is no inventory "test1" of type "pc" for vessel "00000000-0000-0000-0000-000000000002" of customer "00000000-0000-0000-0000-000000000003"
        When I delete inventory "test1" of type "pc" for vessel "00000000-0000-0000-0000-000000000002" of customer "00000000-0000-0000-0000-000000000003"
        Then Inventory "test1" of type "pc" for vessel "00000000-0000-0000-0000-000000000002" of customer "00000000-0000-0000-0000-000000000003" does not exist
