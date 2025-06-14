import {createRouter, createWebHistory} from 'vue-router'
import HomeView from '@/components/HomeView.vue'
import {getToken} from "@/utils.ts";

const router = createRouter({
    history: createWebHistory(import.meta.env.BASE_URL),
    routes: [
        {
            path: '/',
            name: 'home',
            component: HomeView,
            meta: {requiresGuest: true}
        },
        {
            path: '/register',
            name: 'register',
            component: () => import('@/components/Register.vue'),
            meta: {requiresGuest: true}
        },
        {
            path: '/poems',
            name: 'poems',
            // route level code-splitting
            // this generates a separate chunk (About.[hash].js) for this route
            // which is lazy-loaded when the route is visited.
            component: () => import('@/components/GetPoems.vue'),
            meta: {requiresAuth: true},
        },
        {
            path: '/pictures',
            name: 'pictures',
            component: () => import('@/components/GetPictures.vue'),
            meta: {requiresAuth: true},

        },
        {
            path: '/profile',
            name: 'profile',
            component: () => import('@/components/Profile.vue'),
            meta: {requiresAuth: true},
        }
    ],
})

router.beforeEach((to, from, next) => {
    let token = getToken('jwt');
    if (to.meta.requiresAuth && !token) {
        console.log('unauthorized');
        next('/');
    } else if (to.meta.requiresGuest && token) {
        next('/profile'); // or any route for logged-in users
    } else {
        next();
    }
});

export default router
