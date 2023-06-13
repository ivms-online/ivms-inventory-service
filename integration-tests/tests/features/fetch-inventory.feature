##
# This file is part of the IVMS Online.
#
# @copyright 2023 © by Rafał Wrzeszcz - Wrzasq.pl.
##

Feature: Inventory management

    Scenario: Fetching inventory
        Given There is an inventory "test2" of type "pc" for vessel "00000000-0000-0000-0000-000000000006" of customer "00000000-0000-0000-0000-000000000007" with serial number "qwertp", AWS instance ID "abch" and creation date "2011-01-30T14:58:00+01:00"
        When I fetch inventory "test2" of type "pc" for vessel "00000000-0000-0000-0000-000000000006" of customer "00000000-0000-0000-0000-000000000007"
        Then I can read inventory type as "pc"
        And I can read inventory ID as "test2"
        And I can read inventory serial number as "qwertp"
        And I can read inventory AWS instance ID as "abch"
        And I can read inventory creation date as "2011-01-30T14:58:00+01:00"

    Scenario: Fetching non-existing inventory
        Given There is no inventory "test3" of type "pc" for vessel "00000000-0000-0000-0000-000000000008" of customer "00000000-0000-0000-0000-000000000009"
        When I fetch inventory "test3" of type "pc" for vessel "00000000-0000-0000-0000-000000000008" of customer "00000000-0000-0000-0000-000000000009"
        Then I get "Inventory not found." API error response
