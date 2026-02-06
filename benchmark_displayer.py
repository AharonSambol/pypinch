from rich.console import Console
from rich.table import Table
from rich.text import Text


def display_benchmark(
    results,
    *,
    name_key="name",
    metrics=None,
    sort_by=None,
):
    """
    results: list[dict]
        Example:
        [
            {"name": "func_a", "time_ms": 12.3, "mem_mb": 45},
            ...
        ]

    metrics: dict[str, dict]
        Metric configuration:
        {
            "time_ms": {
                "label": "Time (ms)",
                "higher_is_better": False,
                "format": "{:.2f}",
            },
            "mem_mb": {
                "label": "Memory (MB)",
                "higher_is_better": False,
                "format": "{:.1f}",
            },
        }

    sort_by: str | None
        Which metric to sort by
    """

    if not results:
        return

    # console = Console()
    console = Console(force_terminal=True, color_system="truecolor")

    # Auto infer metrics if not supplied
    if metrics is None:
        sample = results[0]
        metrics = {
            k: {
                "label": k,
                "higher_is_better": False,
                "format": "{}",
            }
            for k in sample
            if k != name_key
        }

    # Gather per-metric ranges
    ranges = {}
    for metric in metrics:
        values = [r[metric] for r in results if metric in r]
        ranges[metric] = (min(values), max(values))

    def color_scale(value, vmin, vmax, higher_is_better):
        if vmax == vmin:
            return "white"

        ratio = (value - vmin) / (vmax - vmin)

        # If lower is better → invert scale
        if higher_is_better:
            ratio = 1 - ratio

        if ratio < 0.33:
            return "green"
        elif ratio < 0.66:
            return "yellow"
        return "red"

    # Sorting
    if sort_by:
        higher_is_better = metrics[sort_by].get("higher_is_better", False)
        results = sorted(
            results,
            key=lambda r: r[sort_by],
            reverse=higher_is_better,
        )

    # Build table
    table = Table(header_style="bold")
    table.add_column(name_key.capitalize(), style="bold")

    for metric, cfg in metrics.items():
        table.add_column(cfg.get("label", metric), justify="right")

    # Populate rows
    for r in results:
        row = [str(r[name_key])]

        for metric, cfg in metrics.items():
            value = r[metric]
            vmin, vmax = ranges[metric]

            color = color_scale(
                value,
                vmin,
                vmax,
                cfg.get("higher_is_better", False),
            )

            fmt = cfg.get("format", "{}")
            row.append(Text(fmt.format(value), style=color))

        table.add_row(*row)

    console.print(table)
