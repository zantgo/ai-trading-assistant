/**
 * Deterministic identicon generator.
 * Produces a 5×5 mirrored-grid SVG avatar from a seed string (user_name).
 * No external dependencies — pure TypeScript with Web Crypto hash.
 */

const GRID = 5;
const CELL = 10;
const PADDING = 4;
const SVG_SIZE = GRID * CELL + PADDING * 2;

function hashString(seed: string): number[] {
    // Simple but effective DJB2 hash producing deterministic bytes
    const bytes: number[] = [];
    let hash = 5381;
    for (let i = 0; i < seed.length; i++) {
        hash = ((hash << 5) + hash) + seed.charCodeAt(i);
        hash = hash & 0xffffffff;
    }
    // Generate enough bytes for grid + color (expand the hash)
    let state = hash >>> 0;
    for (let i = 0; i < 16; i++) {
        state = ((state * 1103515245) + 12345) >>> 0;
        bytes.push(state & 0xff);
    }
    return bytes;
}

/** Pick a hue based on the first hash byte, return HSL with fixed saturation/lightness. */
function pickColor(bytes: number[]): string {
    const hue = Math.round((bytes[0] / 255) * 360);
    const sat = 55 + (bytes[1] % 20); // 55-75%
    const light = 45 + (bytes[2] % 15); // 45-60%
    return `hsl(${hue}, ${sat}%, ${light}%)`;
}

function pickBackground(bytes: number[]): string {
    const lightness = 8 + (bytes[3] % 6);
    const hue = Math.round((bytes[0] / 255) * 360);
    return `hsl(${hue}, 15%, ${lightness}%)`;
}

/**
 * Generate a deterministic identicon SVG as a data URI.
 * @param seed - The string to hash (e.g. user_name)
 * @param size - Rendered size in pixels (default 80)
 * @returns SVG data URI string
 */
export function generateIdenticonSvg(seed: string, size: number = 80): string {
    if (!seed || seed.trim().length === 0) {
        return generateIdenticonSvg('default', size);
    }

    const bytes = hashString(seed.trim().toLowerCase());
    const fg = pickColor(bytes);
    const bg = pickBackground(bytes);
    const cells: boolean[] = [];

    // Generate 5×3 left-half grid (right half mirrors)
    for (let y = 0; y < GRID; y++) {
        for (let x = 0; x < 3; x++) {
            const idx = y * 3 + x;
            cells.push(bytes[5 + idx] > 127);
        }
    }

    let rects = '';
    for (let y = 0; y < GRID; y++) {
        for (let x = 0; x < 3; x++) {
            if (cells[y * 3 + x]) {
                // Left side
                const lx = PADDING + x * CELL;
                const ly = PADDING + y * CELL;
                rects += `<rect x="${lx}" y="${ly}" width="${CELL}" height="${CELL}" rx="2"/>`;
                // Mirror right side (reverse column index)
                const rx = PADDING + (GRID - 1 - x) * CELL;
                rects += `<rect x="${rx}" y="${ly}" width="${CELL}" height="${CELL}" rx="2"/>`;
            }
        }
    }

    const svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${SVG_SIZE} ${SVG_SIZE}" width="${size}" height="${size}"><rect width="${SVG_SIZE}" height="${SVG_SIZE}" fill="${bg}" rx="4"/><g fill="${fg}">${rects}</g></svg>`;

    return `data:image/svg+xml,${encodeURIComponent(svg)}`;
}
