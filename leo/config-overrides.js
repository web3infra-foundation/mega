const path = require(`path`);
const alias = require(`./aliases`);
const { aliasWebpack } = require('react-app-alias');

const SRC = `./src`;

/**
 * @description Create aliases for the paths
 */
const aliases = alias(SRC);

/**
 * @description Resolve the aliases to absolute paths
 */
const resolvedAliases = Object.fromEntries(
	Object.entries(aliases).map(([key, value]) => [key, path.resolve(__dirname, value)])
);

/**
 * @description Options for the aliasWebpack plugin
 */
const options = {
	alias: resolvedAliases
};

/**
 * @description Override the webpack config
 * @param {*} config 
 * @returns 
 */
module.exports = function override(config) {
	config.ignoreWarnings = [{ message: /Failed to parse source map/ }];

	return aliasWebpack(options)(config);
};
