import * as mm from 'micromatch';
import { resolve, sep } from 'path';
import { CreateFilter } from './pluginutils';
import ensureArray from './utils/ensureArray';

const createFilter: CreateFilter = function createFilter(include?, exclude?) {
	const getMatcher = (id: string | RegExp) =>
		id instanceof RegExp
			? id
			: {
					test: mm.matcher(
						resolve(id)
							.split(sep)
							.join('/')
					)
			  };

	const includeMatchers = ensureArray(include).map(getMatcher);
	const excludeMatchers = ensureArray(exclude).map(getMatcher);

	return function(id: string | any): boolean {
		if (typeof id !== 'string') return false;
		if (/\0/.test(id)) return false;

		id = id.split(sep).join('/');

		for (let i = 0; i < excludeMatchers.length; ++i) {
			const matcher = excludeMatchers[i];
			if (matcher.test(id)) return false;
		}

		for (let i = 0; i < includeMatchers.length; ++i) {
			const matcher = includeMatchers[i];
			if (matcher.test(id)) return true;
		}

		return !includeMatchers.length;
	};
};

export { createFilter as default };
