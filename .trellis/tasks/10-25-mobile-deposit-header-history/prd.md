# Deposit history header action

Move the existing mobile deposit order-history entry into PageHeader's right
action slot. Use the shared 44px icon button and localized accessible label.
Keep the existing named history route, authenticated visibility and busy-state
protection; do not show a redundant action on history itself. Remove the panel's
history action and retain the payment-window hint without an empty toolbar.

Scope: production mobile deposit view/panel and matching browser fixture.
Validation: mobile release gate and narrow-screen header rendering/navigation.
No backend, payment behavior, global header styling or remote push changes.
