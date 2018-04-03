import Route from '@ember/routing/route';
import ajax from 'ember-fetch/ajax';
import { serializeQueryParams } from 'ember-fetch/mixins/adapter-fetch';

/**
 * This route will be called from the GitHub OAuth flow once the user has
 * accepted or rejected the data access permissions. It will forward the
 * temporary `code` received from the GitHub API to our own API server which
 * will exchange it for an access token.
 *
 * After the exchange the API will return an API token and the user information.
 * The result will be stored and the popup window closed. The `/login` route
 * will then continue to evaluate the response.
 *
 * @see https://developer.github.com/v3/oauth/#github-redirects-back-to-your-site
 * @see `/login` route
 */
export default Route.extend({
    async beforeModel(transition) {
        try {
            let queryParams = serializeQueryParams(transition.queryParams);
            let data = await ajax(`/authorize?${queryParams}`);
            let item = JSON.stringify({ ok: true, data });
            if (window.opener) {
                window.opener.github_response = item;
            }
        } catch(data) {
            let item = JSON.stringify({ ok: false, data });
            if (window.opener) {
                window.opener.github_response = item;
            }
        } finally {
            window.close();
        }
    }
});
