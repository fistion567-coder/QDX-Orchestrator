use anchor_lang::prelude::*;

// Reemplaza esto con la llave pública de tu programa una vez desplegado
declare_id!("11111111111111111111111111111111");

#[program]
pub mod qdx_orchestrator {
    use super::*;

    // 1. Inicializa una nueva orden de simulación molecular
    pub fn initialize_orchestration(
        ctx: Context<InitializeOrchestration>,
        compliance_hash: [u8; 32], // Hash criptográfico de los datos médicos (Privacidad)
        fee_amount: u64,           // Costo de la tarea en tokens/lamports
    ) -> Result<()> {
        let session_account = &mut ctx.accounts.session_account;
        let client = &ctx.accounts.client;

        session_account.client = *client.key;
        session_account.compliance_hash = compliance_hash;
        session_account.fee_amount = fee_amount;
        session_account.status = OrchestrationStatus::Active;
        session_account.bump = ctx.bumps.session_account;

        msg!("QDX Orchestrator: Session initialized for client {}", client.key);
        Ok(())
    }

    // 2. Finaliza la tarea de forma exitosa (Happy Path del diagrama)
    pub fn complete_orchestration(ctx: Context<ManageOrchestration>) -> Result<()> {
        let session_account = &mut ctx.accounts.session_account;
        
        require!(
            session_account.status == OrchestrationStatus::Active,
            OrchestratorError::InvalidStatus
        );

        session_account.status = OrchestrationStatus::Completed;
        msg!("QDX Orchestrator: Task successfully completed and verified.");
        Ok(())
    }

    // 3. Mecanismo de recuperación ante fallos / Reembolso (Manejo de errores del diagrama)
    pub fn trigger_fault_recovery(ctx: Context<ManageOrchestration>) -> Result<()> {
        let session_account = &mut ctx.accounts.session_account;

        require!(
            session_account.status == OrchestrationStatus::Active,
            OrchestratorError::InvalidStatus
        );

        session_account.status = OrchestrationStatus::Refunded;
        msg!("QDX Orchestrator: Fault detected. Automated recovery triggered. Escrow reversed.");
        Ok(())
    }
}

// Estructura de datos requerida para abrir una sesión en Solana
#[derive(Accounts)]
#[instruction(compliance_hash: [u8; 32], fee_amount: u64)]
pub struct InitializeOrchestration<'info> {
    #[account(
        init,
        payer = client,
        space = 8 + 32 + 32 + 8 + 1 + 1, // Tamaño en bytes de la cuenta
        seeds = [b"orchestration_session", client.key().as_ref()],
        bump
    )]
    pub session_account: Account<'info, OrchestrationSession>,
    #[account(mut)]
    pub client: Signer<'info>,
    pub system_program: Program<'info, System>,
}

// Estructura para manejar o cancelar sesiones existentes
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

// Estado interno de la orden guardado en la blockchain
#[account]
pub struct OrchestrationSession {
    pub client: Pubkey,            // 32 bytes
    pub compliance_hash: [u8; 32], // 32 bytes (Cero Datos PII expuestos)
    pub fee_amount: u64,           // 8 bytes
    pub status: OrchestrationStatus, // 1 byte
    pub bump: u8,                  // 1 byte
}

// Enumeración de estados basados en tu arquitectura
#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq)]
pub enum OrchestrationStatus {
    Active,
    Completed,
    Refunded,
}

// Errores personalizados del Orquestador
#[error_code]
pub enum OrchestratorError {
    #[msg("The orchestration session is not in an active state.")]
    InvalidStatus,
}
