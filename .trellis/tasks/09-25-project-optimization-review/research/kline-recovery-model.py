"""Model the reviewed recovery branches; no Rust or database execution."""
import json


def simulate(fail_first_aggregate):
    roots = set()
    aggregates = set()
    checkpoint = 0
    status = 'idle'
    end_exclusive = 5
    now = 10
    trace = []
    for attempt in range(2):
        # Candidate predicate and half-open strategy range use minute slots.
        due = checkpoint < now - 1 and (
            checkpoint + 1 < end_exclusive
            or (status != 'live' and checkpoint < end_exclusive)
        )
        if not due:
            break
        until = min(now - 1, end_exclusive - 1)
        scanned = list(range(checkpoint, until + 1))[:500]
        missing = [slot for slot in scanned if slot not in roots]
        if missing:
            roots.update(missing)
            if attempt == 0 and fail_first_aggregate:
                status = 'failed'
                trace.append({'attempt': attempt + 1, 'roots': len(roots),
                              'aggregates': len(aggregates), 'status': status})
                continue
            if all(slot in roots for slot in range(5)):
                aggregates.add(('5m', 0))
        checkpoint = scanned[-1]
        status = 'live' if checkpoint >= until else 'catching_up'
        trace.append({'attempt': attempt + 1, 'roots': len(roots),
                      'aggregates': len(aggregates), 'status': status})
    selected_next_round = checkpoint < now - 1 and (
        checkpoint + 1 < end_exclusive
        or (status != 'live' and checkpoint < end_exclusive)
    )
    return {'roots': len(roots), 'aggregates': len(aggregates),
            'checkpoint': checkpoint, 'status': status,
            'selected_next_round': selected_next_round, 'trace': trace}


failed = simulate(True)
healthy = simulate(False)
assert failed['roots'] == 5 and failed['aggregates'] == 0
assert failed['status'] == 'live' and failed['checkpoint'] == 4
assert failed['selected_next_round'] is False
assert healthy['aggregates'] == 1 and healthy['status'] == 'live'
print(json.dumps({'aggregate_failure': failed, 'healthy_control': healthy}, indent=2))
print('Two control-flow model scenarios passed; not a database fault-injection test.')
