import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

// Get directory path from command line argument, or default to current directory
const directoryPath = process.argv[2] ? path.resolve(process.argv[2]) : process.cwd();

// Validate if the directory exists
if (!fs.existsSync(directoryPath)) {
    console.error(`Error: Directory "${directoryPath}" does not exist`);
    process.exit(1);
}

console.log(`Starting migration in directory: ${directoryPath}`);

// Create spacing replacements based on your current config
const spacingProperties = [
    'text',                          // text-size
    'p', 'px', 'py', 'pt', 'pr', 'pb', 'pl',   // padding
    'm', 'mx', 'my', 'mt', 'mr', 'mb', 'ml',   // margin
    'gap', 'gap-x', 'gap-y',                   // gap
    'space-x', 'space-y',                      // space
    'w', 'h',                                  // width, height
    'max-w', 'max-h',                          // max-width, max-height
    'min-w', 'min-h',                          // min-width, min-height
    'inset', 'top', 'right', 'bottom', 'left', // positioning
    'translate-x', 'translate-y',              // translate
    'icon-size',                               // icon-size
];

// Create the full replacement mapping
const spacingReplacements = {};
spacingProperties.forEach(prop => {
    // Use a regex to match numeric values
    const regex = new RegExp(`${prop}-(\\d+(?:\\.\\d+)?)$`);
    
    return {
        pattern: regex,
        replace: (match, value) => {
            // Convert the numeric value by dividing by 4
            const newValue = (parseFloat(value) / 4).toString();
            return `${prop}-${newValue}`;
        }
    };
});

// Add new Tailwind v4 class replacements
const tailwindV4Replacements = {
    // Flex utilities
    'flex-shrink-0': 'shrink-0',
    'flex-shrink': 'shrink',
    'flex-grow-0': 'grow-0',
    'flex-grow': 'grow',

    // Text utilities
    'overflow-ellipsis': 'text-ellipsis',

    // Decoration utilities
    'decoration-slice': 'box-decoration-slice',
    'decoration-clone': 'box-decoration-clone',

    // Shadow utilities - be explicit about all variants
    'shadow': 'shadow-sm',
    'shadow-sm': 'shadow-xs',
    
    // Drop shadow utilities
    'drop-shadow': 'drop-shadow-sm',

    // Blur utilities
    'blur-sm': 'blur-xs',
    'blur': 'blur-sm',

    // Rounded utilities - remove the base 'rounded' replacement and be more specific
    'rounded-sm': 'rounded-xs',
    'rounded': 'rounded-sm',
    'rounded-0': 'rounded-none',
    'rounded-b-0': 'rounded-b-none',
    'rounded-t-0': 'rounded-t-none',
    'rounded-l-0': 'rounded-l-none',
    'rounded-r-0': 'rounded-r-none',


    // Outline utilities
    'outline-none': 'outline-hidden',

    // Ring utilities
    'ring': 'ring-3',

    // Text utilities
    'text-10': 'text-xs',
    'text-11': 'text-sm',
    'text-12': 'text-md',
    'text-13': 'text-base',
    'text-14': 'text-lg',
    'text-15': 'text-lg',
    'text-16': 'text-xl',
    'text-17': 'text-xl',
    'text-18': 'text-2xl',
    'text-19': 'text-2xl',
    'text-20': 'text-3xl',
    'text-48': 'text-7xl',

    // Flex utilities
    'flex-0': 'shrink-0',

    // Leading utilities
    'leading-1': 'leading-none',
    'leading-normal': 'leading-[1.5]',
    'leading-tight': 'leading-[1.25]',
    'leading-snug': 'leading-[1.375]',
    'leading-relaxed': 'leading-[1.625]',
    'leading-loose': 'leading-[2]',
    'leading-24': 'leading-[2]',

    'w-md': 'w-xl',
    'max-w-md': 'max-w-xl',

    'w-lg': 'w-2xl',
    'max-w-lg': 'max-w-2xl',

    'w-xl': 'w-3xl',
    'max-w-xl': 'max-w-3xl',
    
    'w-2xl': 'w-4xl',
    'max-w-2xl': 'max-w-4xl',

    'w-3xl': 'w-5xl',
    'max-w-3xl': 'max-w-5xl',

    'w-4xl': 'w-6xl',
    'max-w-4xl': 'max-w-6xl',

    'w-5xl': 'w-7xl',
    'max-w-5xl': 'max-w-7xl',

    'font-500': 'font-medium',
    'font-600': 'font-semibold',
    'font-700': 'font-bold',
    'font-800': 'font-extrabold',
    'font-900': 'font-black',
};

function replaceClassesInFile(filePath) {
    // Skip the migration script itself
    if (filePath === fileURLToPath(import.meta.url)) {
        console.log('Skipping migration script file');
        return;
    }

    fs.readFile(filePath, 'utf8', (err, data) => {
        if (err) {
            console.error(`Error reading file ${filePath}:`, err);
            return;
        }

        let result = data;

        // Handle opacity patterns first
        const opacityPatterns = [
            {
                type: 'bg',
                regex: /bg-((?:slate|gray|zinc|neutral|stone|red|orange|amber|yellow|lime|green|emerald|teal|cyan|sky|blue|indigo|violet|purple|fuchsia|pink|rose|white|black)(?:-\d+)?)\s+(?:group-hover:)?bg-opacity-(\d+)|group-hover:bg-opacity-(\d+)/g,
                replacement: (match, color, opacity1, opacity2) => {
                    if (!color) {
                        // Handle standalone group-hover:bg-opacity case
                        return `group-hover:bg-black/${opacity2}`;
                    }
                    const hover = match.includes('group-hover:') ? 'group-hover:' : '';
                    return `${hover}bg-${color}/${opacity1}`;
                }
            },
            {
                type: 'text',
                regex: /text-((?:slate|gray|zinc|neutral|stone|red|orange|amber|yellow|lime|green|emerald|teal|cyan|sky|blue|indigo|violet|purple|fuchsia|pink|rose|white|black)(?:-\d+)?)\s+text-opacity-(\d+)/g,
                replacement: 'text-$1/$2'
            },
            {
                type: 'border',
                regex: /border-((?:slate|gray|zinc|neutral|stone|red|orange|amber|yellow|lime|green|emerald|teal|cyan|sky|blue|indigo|violet|purple|fuchsia|pink|rose|white|black)(?:-\d+)?)\s+border-opacity-(\d+)/g,
                replacement: 'border-$1/$2'
            },
            {
                type: 'divide',
                regex: /divide-((?:slate|gray|zinc|neutral|stone|red|orange|amber|yellow|lime|green|emerald|teal|cyan|sky|blue|indigo|violet|purple|fuchsia|pink|rose|white|black)(?:-\d+)?)\s+divide-opacity-(\d+)/g,
                replacement: 'divide-$1/$2'
            },
            {
                type: 'ring',
                regex: /ring-((?:slate|gray|zinc|neutral|stone|red|orange|amber|yellow|lime|green|emerald|teal|cyan|sky|blue|indigo|violet|purple|fuchsia|pink|rose|white|black)(?:-\d+)?)\s+ring-opacity-(\d+)/g,
                replacement: 'ring-$1/$2'
            },
            {
                type: 'placeholder',
                regex: /placeholder-((?:slate|gray|zinc|neutral|stone|red|orange|amber|yellow|lime|green|emerald|teal|cyan|sky|blue|indigo|violet|purple|fuchsia|pink|rose|white|black)(?:-\d+)?)\s+placeholder-opacity-(\d+)/g,
                replacement: 'placeholder-$1/$2'
            }
        ];

        // Apply opacity patterns
        opacityPatterns.forEach(({ regex, replacement }) => {
            result = result.replace(regex, replacement);
        });

        // Combine all replacements into a single object
        const allReplacements = {
            ...tailwindV4Replacements,
            ...spacingReplacements
        };

        // Find all className attributes and clsx/cn function calls
        const classPatterns = [
            /className=["']([^"']+)["']/g,
            /className={["']([^"']+)["']}/g,
            /className={`([^`]+)`}/g,
            /(?:clsx|cn)\(([\s\S]*?)\)/g,
            /classes=\{?\{[^}]*[\w]+:\s*["']([^"']+)["'][^}]*\}?\}/g
        ];

        classPatterns.forEach(pattern => {
            result = result.replace(pattern, (match, classString) => {
                // For clsx/cn calls, extract string content from quotes
                if (match.startsWith('clsx(') || match.startsWith('cn(')) {
                    const stringMatches = classString.match(/["']([^"']+)["']/g);
                    if (!stringMatches) return match;

                    return match.replace(/["']([^"']+)["']/g, (m, classes) => {
                        const updatedClasses = classes.split(/\s+/).map(singleClass => {
                            let replaced = singleClass;
                            const colonIndex = singleClass.lastIndexOf(':');
                            
                            if (colonIndex !== -1) {
                                const prefix = singleClass.substring(0, colonIndex + 1);
                                const baseClass = singleClass.substring(colonIndex + 1);
                                
                                // First check tailwindV4Replacements
                                if (tailwindV4Replacements[baseClass]) {
                                    replaced = `${prefix}${tailwindV4Replacements[baseClass]}`;
                                } else {
                                    // Then check spacing patterns
                                    for (const prop of spacingProperties) {
                                        const match = baseClass.match(new RegExp(`^${prop}-(\\d+(?:\\.\\d+)?)$`));
                                        if (match) {
                                            const newValue = (parseFloat(match[1]) / 4).toString();
                                            replaced = `${prefix}${prop}-${newValue}`;
                                            break;
                                        }
                                        // Handle negative values
                                        const negMatch = baseClass.match(new RegExp(`^-${prop}-(\\d+(?:\\.\\d+)?)$`));
                                        if (negMatch) {
                                            const newValue = (parseFloat(negMatch[1]) / 4).toString();
                                            replaced = `${prefix}-${prop}-${newValue}`;
                                            break;
                                        }
                                    }
                                }
                            } else {
                                // First check tailwindV4Replacements
                                if (tailwindV4Replacements[replaced]) {
                                    replaced = tailwindV4Replacements[replaced];
                                } else {
                                    // Then check spacing patterns
                                    for (const prop of spacingProperties) {
                                        const match = replaced.match(new RegExp(`^${prop}-(\\d+(?:\\.\\d+)?)$`));
                                        if (match) {
                                            const newValue = (parseFloat(match[1]) / 4).toString();
                                            replaced = `${prop}-${newValue}`;
                                            break;
                                        }
                                        // Handle negative values
                                        const negMatch = replaced.match(new RegExp(`^-${prop}-(\\d+(?:\\.\\d+)?)$`));
                                        if (negMatch) {
                                            const newValue = (parseFloat(negMatch[1]) / 4).toString();
                                            replaced = `-${prop}-${newValue}`;
                                            break;
                                        }
                                    }
                                }
                            }
                            return replaced;
                        }).join(' ');
                        
                        return m.replace(classes, updatedClasses);
                    });
                }

                // Regular className handling
                let updatedClassString = classString.split(/\s+/).map(singleClass => {
                    let replaced = singleClass;
                    const colonIndex = singleClass.lastIndexOf(':');
                    
                    if (colonIndex !== -1) {
                        const prefix = singleClass.substring(0, colonIndex + 1);
                        const baseClass = singleClass.substring(colonIndex + 1);
                        
                        // First check tailwindV4Replacements
                        if (tailwindV4Replacements[baseClass]) {
                            replaced = `${prefix}${tailwindV4Replacements[baseClass]}`;
                        } else {
                            // Then check spacing patterns
                            for (const prop of spacingProperties) {
                                const match = baseClass.match(new RegExp(`^${prop}-(\\d+(?:\\.\\d+)?)$`));
                                if (match) {
                                    const newValue = (parseFloat(match[1]) / 4).toString();
                                    replaced = `${prefix}${prop}-${newValue}`;
                                    break;
                                }
                                // Handle negative values
                                const negMatch = baseClass.match(new RegExp(`^-${prop}-(\\d+(?:\\.\\d+)?)$`));
                                if (negMatch) {
                                    const newValue = (parseFloat(negMatch[1]) / 4).toString();
                                    replaced = `${prefix}-${prop}-${newValue}`;
                                    break;
                                }
                            }
                        }
                    } else {
                        // First check tailwindV4Replacements
                        if (tailwindV4Replacements[replaced]) {
                            replaced = tailwindV4Replacements[replaced];
                        } else {
                            // Then check spacing patterns
                            for (const prop of spacingProperties) {
                                const match = replaced.match(new RegExp(`^${prop}-(\\d+(?:\\.\\d+)?)$`));
                                if (match) {
                                    const newValue = (parseFloat(match[1]) / 4).toString();
                                    replaced = `${prop}-${newValue}`;
                                    break;
                                }
                                // Handle negative values
                                const negMatch = replaced.match(new RegExp(`^-${prop}-(\\d+(?:\\.\\d+)?)$`));
                                if (negMatch) {
                                    const newValue = (parseFloat(negMatch[1]) / 4).toString();
                                    replaced = `-${prop}-${newValue}`;
                                    break;
                                }
                            }
                        }
                    }
                    return replaced;
                }).join(' ');

                return match.replace(classString, updatedClassString);
            });
        });

        fs.writeFile(filePath, result, 'utf8', (err) => {
            if (err) {
                console.error(`Error writing file ${filePath}:`, err);
            } else {
                console.log(`Updated file: ${filePath}`);
            }
        });
    });
}

function traverseDirectory(directory) {
    fs.readdir(directory, (err, files) => {
        if (err) {
            console.error(`Error reading directory ${directory}:`, err);
            return;
        }

        files.forEach((file) => {
            const filePath = path.join(directory, file);
            fs.stat(filePath, (err, stats) => {
                if (err) {
                    console.error(`Error getting stats of file ${filePath}:`, err);
                    return;
                }

                if (stats.isDirectory()) {
                    traverseDirectory(filePath);
                } else if (filePath.endsWith('.js') || filePath.endsWith('.jsx') || filePath.endsWith('.ts') || filePath.endsWith('.tsx')) {
                    replaceClassesInFile(filePath);
                }
            });
        });
    });
}


traverseDirectory(directoryPath);