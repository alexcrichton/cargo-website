module.exports = {
    root: true,
    parserOptions: {
        ecmaVersion: 2017,
        sourceType: 'module',
        ecmaFeatures: {
            'experimentalObjectRestSpread': true,
        },
    },
    extends: [
        'eslint:recommended',
        'plugin:ember-suave/recommended',
		'plugin:ember/recommended'
    ],
    env: {
        browser: true,
    },
    rules: {
        'arrow-parens': 'off',
        'brace-style': 'off',
        'camelcase': 'off',
        'comma-dangle': 'off',
        'dot-notation': 'off',
        'indent': ['error', 4],
        'operator-linebreak': 'off',
        'quotes': ['error', 'single', {
            'allowTemplateLiterals': true,
            'avoidEscape': true,
        }],

        'ember-suave/no-const-outside-module-scope': 'off',
        'ember-suave/no-direct-property-access': 'off',
        'ember-suave/require-access-in-comments': 'off',
		'ember/no-jquery': 'error',
		'ember/avoid-leaking-state-in-ember-objects': 'off',
		'ember/no-capital-letters-in-routes': 'off',
		'ember/no-on-calls-in-components': 'off'
    },
};
