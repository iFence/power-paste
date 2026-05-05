import { createRouter, createWebHashHistory } from 'vue-router'

const routes = [
  {
    path: '/',
    name: 'home',
    component: { template: '<div />' },
  },
  {
    path: '/lan-transfer',
    name: 'lanTransfer',
    component: { template: '<div />' },
  },
]

export const router = createRouter({
  history: createWebHashHistory(),
  routes,
})
