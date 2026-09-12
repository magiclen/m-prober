export const usageColor = (ratio: number): string => {
    if (ratio >= 0.9) {
        return "red";
    }
    if (ratio >= 0.7) {
        return "yellow";
    }
    return ratio >= 0.5 ? "green" : "blue";
};

export const segmentWidths = (values: number[], total: number): number[] => {
    let remaining = 100;
    const widths: number[] = [];
    for (const value of values) {
        const width = Math.min(remaining, Math.max(0, total > 0 ? (value * 100) / total : 0));
        widths.push(width);
        remaining -= width;
    }
    return widths;
};
