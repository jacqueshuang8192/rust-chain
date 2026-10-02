"""Model observed lock order, not a MySQL integration test."""
from collections import deque


def explore(plans):
    initial = ((0, 0), ())
    queue = deque([initial])
    visited = {initial}
    deadlocks = []
    completed = 0
    while queue:
        positions, locks = queue.popleft()
        if all(positions[i] == len(plans[i]) for i in range(2)):
            completed += 1
            continue
        successors = []
        waiting = []
        for owner, plan in enumerate(plans):
            position = positions[owner]
            if position == len(plan):
                continue
            resource, mode = plan[position]
            conflicts = [(r, holder, held) for r, holder, held in locks
                         if r == resource and holder != owner
                         and (held == 'X' or mode == 'X')]
            if conflicts:
                waiting.append((owner, resource, mode, conflicts))
                continue
            next_positions = list(positions)
            next_positions[owner] += 1
            if mode == 'COMMIT':
                next_locks = tuple(lock for lock in locks if lock[1] != owner)
            else:
                next_locks = tuple(sorted((*locks, (resource, owner, mode))))
            successors.append((tuple(next_positions), next_locks))
        if not successors:
            deadlocks.append(waiting)
        for state in successors:
            if state not in visited:
                visited.add(state)
                queue.append(state)
    return len(visited), completed, deadlocks


recharge = [('user:U', 'X'), ('agent:1', 'S'), ('agent:2', 'S'), ('', 'COMMIT')]
legacy_other = [('user:V', 'X'), ('agent:2', 'X'), ('agent:1', 'X'), ('', 'COMMIT')]
legacy_same = [('user:U', 'X'), ('agent:2', 'X'), ('agent:1', 'X'), ('', 'COMMIT')]
ordered_other = [('user:V', 'X'), ('agent:1', 'X'), ('agent:2', 'X'), ('', 'COMMIT')]

for name, other, expected in [
    ('legacy-other-user', legacy_other, True),
    ('same-user-control', legacy_same, False),
    ('consistent-order-control', ordered_other, False),
]:
    states, completed, deadlocks = explore((recharge, other))
    assert bool(deadlocks) == expected, (name, deadlocks)
    assert completed > 0
    print(f'{name}: states={states}, completed={completed}, deadlocks={len(deadlocks)}')
    if deadlocks:
        print(f'  wait_cycle={deadlocks[0]}')
print('Three lock-model assertions passed; no database operations performed.')
