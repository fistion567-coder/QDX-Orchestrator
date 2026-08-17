use anchor_lang::prelude::*;
use anchor_spl::token::{self, Burn, Token, TokenAccount};

declare_id!("11111111111111111111111111111111");

#[program]
pub mod qdx_orchestrator {
    use super::*;

    // 1. Inicializa la orden y procesa el cobro/quema de comisiones en $QDX
    pub fn initialize_orchestration(
        ctx: Context<InitializeOrchestration>,
        compliance_hash: [u8; 32],
        fee_amount: u64,
    ) -> Result<()> {
        let session_account = &mut ctx.accounts.session_account;
        let client = &ctx.accounts.client;

        // Calcular porcentajes de Tokenomics: 30% se quema, 70% va al pozo de recompensa
        let burn_amount = (fee_amount * 30) / 100;
        let reward_amount = fee_amount - burn_amount;

        // Ejecutar la quema (Burn) del 30% de los tokens QDX de la tarifa
        let cpi_program_burn = ctx.accounts.token_program.to_account_info();
        let cpi_accounts_burn = Burn {
            mint: ctx.accounts.qdx_mint.to_account_info(),
            from: ctx.accounts.client_qdx_vault.to_account_info(),
            authority: client.to_account_info(),
        };
        let cpi_ctx_burn = CpiContext::new(cpi_program_burn, cpi_accounts_burn);
        token::burn(cpi_ctx_burn, burn_amount)?;

        // Transferir el 70% restante al pozo de validadores/recompensas del orquestador
        let cpi_program_transfer = ctx.accounts.token_program.to_account_info();
        let cpi_accounts_transfer = token::Transfer {
            from: ctx.accounts.client_qdx_vault.to_account_info(),
            to: ctx.accounts.orchestrator_rewards_vault.to_account_info(),
            authority: client.to_account_info(),
        };
        let cpi_ctx_transfer = CpiContext::new(cpi_program_transfer, cpi_accounts_transfer);
        token::transfer(cpi_ctx_transfer, reward_amount)?;

        // Guardar estado en la cuenta de la sesión
        session_account.client = *client.key;
        session_account.compliance_hash = compliance_hash;
        session_account.fee_amount = fee_amount;
        session_account.status = OrchestrationStatus::Active;
        session_account.bump = ctx.bumps.session_account;

        msg!("QDX Orchestrator: Session active. Fees processed: 30% burned, 70% escrowed.");
        Ok(())
    }

    pub fn complete_orchestration(ctx: Context<ManageOrchestration>) -> Result<()> {
        let session_account = &mut ctx.accounts.session_account;
        require!(session_account.status == OrchestrationStatus::Active, OrchestratorError::InvalidStatus);
        session_account.status = OrchestrationStatus::Completed;
        msg!("QDX Orchestrator: Task successfully completed.");
        Ok(())
    }

    pub fn trigger_fault_recovery(ctx: Context<ManageOrchestration>) -> Result<()> {
        let session_account = &mut ctx.accounts.session_account;
        require!(session_account.status == OrchestrationStatus::Active, OrchestratorError::InvalidStatus);
        session_account.status = OrchestrationStatus::Refunded;
        msg!("QDX Orchestrator: Fault detected. Automated recovery triggered.");
        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(compliance_hash: [u8; 32], fee_amount: u64)]
pub struct InitializeOrchestration<'info> {
    #[account(
        init,
        payer = client,
        space = 8 + 32 + 32 + 8 + 1 + 1,
        seeds = [b"orchestration_session", client.key().as_ref()],
        bump
    )]
    pub session_account: Account<'info, OrchestrationSession>,
    
    #[account(mut)]
    pub client: Signer<'info>,
    
    #[account(mut)]
    pub qdx_mint: Account<'info, token::Mint>, // El token oficial $QDX
    
    #[account(mut)]
    pub client_qdx_vault: Account<'info, TokenAccount>, // Bóveda del cliente médico
    
    #[account(mut)]
    pub orchestrator_rewards_vault: Account<'info, TokenAccount>, // Bóveda de recompensas del proyecto
    
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ManageOrchestration<'info> {
    #[account(
        mut,
        seeds = [b"orchestration_session", client.key().as_ref()],
        bump = session_account.bump,
        has_one = client
    )]
    pub session_account: Account<'info, OrchestrationSession>,
    pub client: Signer<'info>,
}

#[account]
pub struct OrchestrationSession {
    pub client: Pubkey,
    pub compliance_hash: [u8; 32],
    pub fee_amount: u64,
    pub status: OrchestrationStatus,
    pub bump: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq)]
pub enum OrchestrationStatus {
    Active,
    Completed,
    Refunded,
}

#[error_code]
pub enum OrchestratorError {
    #[msg("The orchestration session is not in an active state.")]
    InvalidStatus,
}

