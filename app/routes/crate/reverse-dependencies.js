import Route from '@ember/routing/route';

export default Route.extend({
    queryParams: {
        page: { refreshModel: true },
    },

    model(params) {
        params.reverse = true;
        params.crate = this.modelFor('crate');

        return this.store.query('dependency', params);
    }
});
