/// <reference types="cypress" />

describe('Full flow test', () => {
    it('passes', () => {
        cy.visit('http://localhost:5173/')
        cy.get('input[name="question"]').type('hello{enter}')
        cy.get('textarea[name="answer"]').should('have.value', 'Hello from Node.js backend! - hello')
    })
})